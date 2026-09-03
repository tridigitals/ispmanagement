<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import AttentionPanel from '$lib/components/ds/AttentionPanel.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import TableSkeleton from '$lib/components/ds/TableSkeleton.svelte';
  import type { AttentionItem } from '$lib/components/ds/AttentionPanel.svelte';
  import {
    formatRupiah,
    formatCompactRupiah,
    formatDate,
    formatPercent,
  } from '$lib/components/ds/format';
  import { fetchAllPages } from '$lib/utils/fetchAllPages';
  import type { Invoice } from '$lib/api/types';

  let loading = $state(true);
  let err = $state('');
  let invoices = $state<Invoice[]>([]);
  /** false kalau maxPages memotong pengambilan — angka jadi minimum, bukan pasti. */
  let invoicesComplete = $state(true);
  let customerTotal = $state(0);
  let customerActive = $state(0);
  let pendingInstall = $state(0);
  let pppoeTotal = $state(0);
  let pppoeDisabled = $state(0);
  let loadedAt = $state('');

  const billingRead = $derived($can('read', 'billing') || $can('manage', 'billing'));

  /* Agregat dihitung sekali di sini, bukan diulang di markup. */
  const money = $derived.by(() => {
    let issued = 0;
    let paid = 0;
    let unpaid = 0;
    let paidCount = 0;
    let unpaidCount = 0;
    let overdueCount = 0;
    const now = Date.now();

    for (const inv of invoices) {
      const amt = Number(inv.amount) || 0;
      issued += amt;
      if (inv.status === 'paid') {
        paid += amt;
        paidCount++;
      } else {
        unpaid += amt;
        unpaidCount++;
        if (inv.due_date && new Date(inv.due_date).getTime() < now) overdueCount++;
      }
    }

    const rate = issued > 0 ? (paid / issued) * 100 : 0;
    return { issued, paid, unpaid, paidCount, unpaidCount, overdueCount, rate };
  });

  /* Daftar pekerjaan. Hanya masalah yang benar-benar ada yang muncul —
     panel kosong berarti tidak ada yang perlu ditangani, bukan bug. */
  const attention = $derived.by(() => {
    const items: AttentionItem[] = [];

    if (money.overdueCount > 0) {
      items.push({
        icon: 'receipt',
        title: 'Tagihan jatuh tempo belum dibayar',
        detail: `${money.overdueCount} invoice · ${formatRupiah(money.unpaid)} · ${formatPercent(money.rate)} tingkat pembayaran`,
        action: `Tinjau ${money.overdueCount} tagihan`,
        href: '/admin/invoices?status=pending',
        severity: 'high',
      });
    }

    if (pppoeDisabled > 0) {
      const share = pppoeTotal > 0 ? ((pppoeDisabled / pppoeTotal) * 100).toFixed(0) : '0';
      items.push({
        icon: 'key',
        title: 'Akun PPPoE dinonaktifkan',
        detail: `${pppoeDisabled} dari ${pppoeTotal} akun (${share}%) tidak bisa dial`,
        action: 'Buka daftar PPPoE',
        href: '/admin/network/pppoe?status=disabled',
        severity: pppoeDisabled > pppoeTotal / 2 ? 'high' : 'medium',
      });
    }

    if (pendingInstall > 0) {
      items.push({
        icon: 'clipboard',
        title: 'Instalasi menunggu jadwal',
        detail: `${pendingInstall} pelanggan sudah terdaftar tapi belum aktif`,
        action: 'Atur jadwal teknisi',
        href: '/admin/network/installations',
        severity: 'medium',
      });
    }

    return items;
  });

  const soonest = $derived(
    invoices
      .filter((i) => i.status !== 'paid')
      .slice()
      .sort((a, b) => new Date(a.due_date).getTime() - new Date(b.due_date).getTime())
      .slice(0, 8),
  );

  /** Ambil nama pelanggan dari deskripsi invoice: "Customer X - Paket ...". */
  function customerOf(inv: Invoice): string {
    const d = inv.description ?? '';
    const m = d.match(/^Customer\s+(.+?)\s+-\s+/);
    return m ? m[1] : (inv.invoice_number ?? '—');
  }

  function packageOf(inv: Invoice): string {
    const m = (inv.description ?? '').match(/-\s+([^(]+?)\s*\(/);
    return m ? m[1].trim() : '—';
  }

  function isOverdue(inv: Invoice): boolean {
    return inv.status !== 'paid' && !!inv.due_date && new Date(inv.due_date).getTime() < Date.now();
  }

  async function load() {
    loading = true;
    err = '';

    const jobs: Promise<unknown>[] = [
      api.customers
        .summary()
        .then((s) => {
          customerTotal = s.total;
          customerActive = s.active;
          pendingInstall = s.pending_installation;
        })
        .catch((e) => console.warn('summary pelanggan gagal', e)),
    ];

    if (billingRead) {
      jobs.push(
        fetchAllPages<Invoice>(
          (page, per_page) =>
            api.payment.listCustomerPackageInvoices({
              sort_by: 'due_date',
              sort_dir: 'asc',
              page,
              per_page,
            }),
          { perPage: 100, maxPages: 30 },
        )
          .then((res) => {
            invoices = res.rows;
            invoicesComplete = res.complete;
          })
          .catch((e) => {
            err = 'Gagal memuat tagihan';
            console.warn('invoice gagal', e);
          }),
      );
    }

    if ($can('read', 'pppoe') || $can('manage', 'pppoe')) {
      jobs.push(
        fetchAllPages<{ disabled?: boolean }>(
          (page, per_page) => api.pppoe.accounts.list({ page, per_page }),
          { perPage: 100, maxPages: 30 },
        )
          .then((res) => {
            pppoeTotal = res.total;
            pppoeDisabled = res.rows.filter((r) => r.disabled).length;
          })
          .catch((e) => console.warn('pppoe gagal', e)),
      );
    }

    await Promise.all(jobs);
    loadedAt = new Date().toISOString();
    loading = false;
  }

  onMount(load);
</script>

<AppShell
  title="Ruang kendali"
  badges={{ invoicesOverdue: money.overdueCount, supportOpen: 0 }}
>
  <PageHeader
    eyebrow={loadedAt ? `Diperbarui ${formatDate(loadedAt, true)}` : 'Memuat data'}
    title="Ruang kendali"
    desc="Kerjakan yang menunggu tindakan dulu, angka menyusul di bawah."
  >
    {#snippet actions()}
      <Button icon="refresh" onclick={load} loading={loading}>Muat ulang</Button>
      <Button variant="primary" icon="plus" href="/admin/customers?new=1">Pelanggan baru</Button>
    {/snippet}
  </PageHeader>

  {#if err}
    <div
      role="alert"
      class="mb-4 rounded-lg border border-red-200 bg-red-50 px-3.5 py-2.5 text-base text-red-800"
    >
      {err}
    </div>
  {/if}

  <!-- Pekerjaan dulu, angka kemudian. -->
  {#if loading}
    <div class="mb-5 space-y-2">
      {#each Array(3) as _}
        <div class="skeleton h-16 rounded-xl"></div>
      {/each}
    </div>
  {:else if attention.length > 0}
    <div class="mb-5">
      <AttentionPanel items={attention} />
    </div>
  {:else}
    <div
      class="mb-5 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-base text-emerald-800"
    >
      Tidak ada yang perlu tindakan segera.
    </div>
  {/if}

  <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
    <StatTile
      label="Tertunggak"
      value={formatCompactRupiah(money.unpaid)}
      hint="{invoicesComplete ? '' : 'minimal '}{money.unpaidCount} invoice belum dibayar · {money.overdueCount} lewat jatuh tempo"
      tone={money.unpaid > 0 ? 'negative' : 'positive'}
    />
    <StatTile
      label="Terbayar"
      value={formatCompactRupiah(money.paid)}
      hint="{money.paidCount} dari {invoices.length} invoice · {formatPercent(money.rate)} dari {formatCompactRupiah(money.issued)} diterbitkan"
      tone={money.rate >= 50 ? 'positive' : 'warning'}
    />
    <StatTile
      label="Pelanggan aktif"
      value={String(customerActive)}
      hint="dari {customerTotal} terdaftar · {pendingInstall} menunggu instalasi"
    />
    <StatTile
      label="PPPoE bisa dial"
      value={String(pppoeTotal - pppoeDisabled)}
      hint="dari {pppoeTotal} akun · {pppoeDisabled} dinonaktifkan"
      tone={pppoeDisabled > pppoeTotal / 2 ? 'negative' : 'neutral'}
    />
  </div>

  <div class="mt-5">
    <Card title="Jatuh tempo terdekat" padded={false}>
      {#snippet aside()}
        <a
          href="/admin/invoices"
          class="focus-ring rounded text-sm font-medium text-brand-600 hover:text-brand-700"
        >
          Lihat semua {money.unpaidCount}
        </a>
      {/snippet}

      {#if loading}
        <div class="px-4 py-3">
          <TableSkeleton rows={6} cols={5} />
        </div>
      {:else if soonest.length === 0}
        <div class="px-4 py-10 text-center text-base text-ink-500">
          Tidak ada tagihan tertunggak.
        </div>
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full border-collapse text-base">
            <thead>
              <tr class="border-b border-ink-200 bg-ink-50">
                <th class="px-4 py-2 text-left text-sm font-semibold text-ink-500">Pelanggan</th>
                <th class="hidden px-4 py-2 text-left text-sm font-semibold text-ink-500 md:table-cell"
                  >Paket</th
                >
                <th class="px-4 py-2 text-right text-sm font-semibold text-ink-500">Nominal</th>
                <th class="px-4 py-2 text-left text-sm font-semibold text-ink-500">Jatuh tempo</th>
                <th class="px-4 py-2 text-left text-sm font-semibold text-ink-500">Status</th>
              </tr>
            </thead>
            <tbody>
              {#each soonest as inv (inv.id)}
                <tr class="border-b border-ink-100 last:border-0 hover:bg-ink-50">
                  <td class="px-4 py-2.5 font-medium text-ink-900">{customerOf(inv)}</td>
                  <td class="hidden px-4 py-2.5 text-ink-500 md:table-cell">{packageOf(inv)}</td>
                  <td class="num px-4 py-2.5 text-right text-ink-900">{formatRupiah(inv.amount)}</td>
                  <td class="num px-4 py-2.5 {isOverdue(inv) ? 'text-red-700' : 'text-ink-500'}">
                    {formatDate(inv.due_date)}
                  </td>
                  <td class="px-4 py-2.5">
                    <Badge
                      status={inv.status}
                      label={isOverdue(inv) ? 'Lewat tempo' : inv.status}
                      tone={isOverdue(inv) ? 'negative' : undefined}
                    />
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <div class="border-t border-ink-200 px-4 py-2 text-sm text-ink-500">
          Menampilkan {soonest.length} dari {money.unpaidCount} tagihan tertunggak · total {formatRupiah(
            money.unpaid,
          )}
        </div>
      {/if}
    </Card>
  </div>
</AppShell>
