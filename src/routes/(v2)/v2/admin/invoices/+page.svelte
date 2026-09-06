<script lang="ts">
  /*
    Tagihan v2 — halaman uang. Versi lama: 1.147 baris, 7.982 karakter CSS.

    Keputusan yang membedakan dari versi lama:

    1. ANGKA AGREGAT DIHITUNG ATAS SELURUH DATA, BUKAN SATU HALAMAN.
       Versi lama menampilkan tabel berpaginasi 25 baris tanpa ringkasan sama
       sekali, jadi tidak ada satu tempat pun di aplikasi yang memberi tahu
       "berapa total piutang". Di sini ringkasan diambil lewat `fetchAllPages`
       (backend clamp per_page ke 100) dan ditandai jujur bila belum lengkap.

    2. FILTER STATUS JADI CHIP BERANGKA. Versi lama memakai dropdown status:
       pengguna harus membuka dropdown untuk tahu pilihannya, dan tidak pernah
       tahu ada berapa banyak per status.

    3. PILIH BANYAK -> KIRIM. Aksi massal tetap ada (bulkSendInvoices), tapi
       tombolnya baru muncul setelah ada baris terpilih, bukan selalu nongkrong
       di toolbar.
  */
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Icon from '$lib/components/ds/Icon.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import TableSkeleton from '$lib/components/ds/TableSkeleton.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import { formatRupiah, formatDate, formatPercent } from '$lib/components/ds/format';
  import { fetchAllPages } from '$lib/utils/fetchAllPages';
  import type { Invoice } from '$lib/api/types';

  type StatusKey = 'all' | 'pending' | 'verification_pending' | 'paid' | 'failed';

  let rows = $state<Invoice[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let statusFilter = $state<StatusKey>('all');
  let loading = $state(true);
  let err = $state('');

  /* Ringkasan seluruh tenant, terpisah dari tabel berpaginasi. */
  let summary = $state({
    count: 0,
    complete: true,
    pending: { n: 0, amount: 0 },
    paid: { n: 0, amount: 0 },
    verification: { n: 0, amount: 0 },
    failed: { n: 0, amount: 0 },
    overdue: { n: 0, amount: 0 },
    /* Aging: piutang yang lewat >90 hari. Terukur di tenant ini 474 dari 476
       piutang sudah >90 hari (tertua 754 hari), jadi tile "jatuh tempo" dan
       "piutang" akan selalu kembar — aging yang benar-benar membedakan. */
    aged90: { n: 0, amount: 0 },
  });
  let summaryLoading = $state(true);

  let selected = $state<Set<string>>(new Set());
  let sending = $state(false);
  let generating = $state(false);
  let notice = $state('');

  /* Resource-nya 'billing', bukan 'payments' — tabel permissions tidak punya baris
     'payments' sama sekali, jadi versi lama silently false untuk semua non-Owner dan
     menyembunyikan tombol aksi dari Admin yang berhak. Halaman legacy memakai
     can('manage','billing'). */
  const canManage = $derived($can('manage', 'billing'));
  const lastPage = $derived(Math.max(1, Math.ceil(total / perPage)));
  const from = $derived(total === 0 ? 0 : (page - 1) * perPage + 1);
  const to = $derived(Math.min(page * perPage, total));

  const billed = $derived(
    summary.pending.amount +
      summary.paid.amount +
      summary.verification.amount +
      summary.failed.amount,
  );
  const collectionRate = $derived(billed === 0 ? 0 : (summary.paid.amount / billed) * 100);

  const chips = $derived([
    { key: 'all' as StatusKey, label: 'Semua', count: summary.count },
    { key: 'pending' as StatusKey, label: 'Belum dibayar', count: summary.pending.n },
    {
      key: 'verification_pending' as StatusKey,
      label: 'Perlu verifikasi',
      count: summary.verification.n,
    },
    { key: 'paid' as StatusKey, label: 'Lunas', count: summary.paid.n },
    { key: 'failed' as StatusKey, label: 'Gagal', count: summary.failed.n },
  ]);

  function isOverdue(inv: Invoice): boolean {
    if (inv.status !== 'pending' && inv.status !== 'verification_pending') return false;
    return !!inv.due_date && new Date(inv.due_date).getTime() < Date.now();
  }

  /** Umur tunggakan dalam hari; negatif berarti belum jatuh tempo. */
  function daysLate(inv: Invoice): number {
    if (!inv.due_date) return 0;
    return Math.floor((Date.now() - new Date(inv.due_date).getTime()) / 86_400_000);
  }

  function toggle(id: string) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selected = next;
  }

  function toggleAll() {
    selected = selected.size === rows.length ? new Set() : new Set(rows.map((r) => r.id));
  }

  async function load() {
    loading = true;
    err = '';
    try {
      const res = await api.payment.listCustomerPackageInvoices({
        status: statusFilter === 'all' ? undefined : statusFilter,
        sort_by: 'due_date',
        sort_dir: 'asc',
        page,
        per_page: perPage,
      });
      rows = res.data ?? [];
      total = res.total ?? 0;
      selected = new Set();
    } catch (e) {
      err = 'Gagal memuat daftar tagihan';
      console.warn('list invoices gagal', e);
    } finally {
      loading = false;
    }
  }

  async function loadSummary() {
    summaryLoading = true;
    try {
      /* Semua halaman, bukan satu: lihat catatan 1 di kepala file. */
      const { rows: all, total: n, complete } = await fetchAllPages<Invoice>((p, per_page) =>
        api.payment.listCustomerPackageInvoices({ page: p, per_page }),
      );

      const bucket = (predicate: (i: Invoice) => boolean) => {
        const list = all.filter(predicate);
        return { n: list.length, amount: list.reduce((s, i) => s + (i.amount ?? 0), 0) };
      };

      summary = {
        count: n,
        complete,
        pending: bucket((i) => i.status === 'pending'),
        paid: bucket((i) => i.status === 'paid'),
        verification: bucket((i) => i.status === 'verification_pending'),
        failed: bucket((i) => i.status === 'failed'),
        overdue: bucket(isOverdue),
        aged90: bucket((i) => isOverdue(i) && daysLate(i) > 90),
      };
    } catch (e) {
      console.warn('ringkasan tagihan gagal', e);
    } finally {
      summaryLoading = false;
    }
  }

  async function sendSelected() {
    if (selected.size === 0) return;
    sending = true;
    notice = '';
    try {
      const res = await api.payment.bulkSendInvoices({
        invoice_ids: [...selected],
        channels: ['email', 'notification'],
        attach_pdf: true,
      });
      const parts = [`terkirim ${res.sent_count}`];
      if (res.skipped_count > 0) parts.push(`dilewati ${res.skipped_count}`);
      if (res.failed_count > 0) parts.push(`gagal ${res.failed_count}`);
      notice = `Dari ${res.total} tagihan: ${parts.join(', ')}.`;
      selected = new Set();
    } catch (e) {
      err = `Gagal mengirim tagihan: ${(e as Error)?.message ?? e}`;
    } finally {
      sending = false;
    }
  }

  async function generateDue() {
    generating = true;
    notice = '';
    try {
      const res = await api.payment.generateDueCustomerPackageInvoices();
      notice =
        `Dibuat ${res.created_count} tagihan baru` +
        (res.skipped_count > 0 ? `, dilewati ${res.skipped_count}` : '') +
        (res.failed_count > 0 ? `, gagal ${res.failed_count}` : '') +
        '.';
      await Promise.all([load(), loadSummary()]);
    } catch (e) {
      err = `Gagal membuat tagihan: ${(e as Error)?.message ?? e}`;
    } finally {
      generating = false;
    }
  }

  function applyChip(key: StatusKey) {
    statusFilter = key;
    page = 1;
    load();
  }

  onMount(() => {
    void load();
    void loadSummary();
  });
</script>

<AppShell title="Tagihan" badges={{ invoicesOverdue: summary.overdue.n }}>
  <PageHeader
    title="Tagihan"
    eyebrow={summaryLoading
      ? 'Menghitung seluruh tagihan…'
      : summary.complete
        ? `${summary.count} tagihan dihitung`
        : `minimal ${summary.count} tagihan (data belum lengkap)`}
    desc="Ringkasan di bawah dihitung atas seluruh tagihan tenant, bukan hanya halaman yang tampil."
  >
    {#snippet actions()}
      {#if canManage}
        <Button icon="refresh" loading={generating} onclick={generateDue}>
          Buat tagihan jatuh tempo
        </Button>
      {/if}
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

  {#if notice}
    <div
      role="status"
      class="mb-4 rounded-lg border border-emerald-200 bg-emerald-50 px-3.5 py-2.5 text-base text-emerald-800"
    >
      {notice}
    </div>
  {/if}

  <!-- Ringkasan uang: 4 angka yang menentukan keputusan hari ini. -->
  <Card class="mb-4">
    <div class="grid grid-cols-2 gap-5 lg:grid-cols-4">
      <StatTile
        label="Piutang"
        value={formatRupiah(summary.pending.amount + summary.verification.amount)}
        hint={summaryLoading
          ? 'menghitung…'
          : `${summary.pending.n + summary.verification.n} belum lunas · ${summary.overdue.n} jatuh tempo`}
        tone="negative"
      />
      <StatTile
        label="Lewat 90 hari"
        value={formatRupiah(summary.aged90.amount)}
        hint={summaryLoading
          ? 'menghitung…'
          : `${summary.aged90.n} dari ${summary.overdue.n} tagihan jatuh tempo`}
        tone="negative"
      />
      <StatTile
        label="Terbayar"
        value={formatRupiah(summary.paid.amount)}
        hint={summaryLoading ? 'menghitung…' : `${summary.paid.n} tagihan lunas`}
        tone="positive"
      />
      <StatTile
        label="Tingkat penagihan"
        value={formatPercent(collectionRate)}
        hint={summaryLoading ? 'menghitung…' : `dari ${formatRupiah(billed)} diterbitkan`}
        tone={collectionRate < 50 ? 'negative' : 'positive'}
      />
    </div>
  </Card>

  <!-- Filter + aksi massal -->
  <div class="mb-3 flex flex-wrap items-center gap-2">
    {#each chips as chip (chip.key)}
      <button
        onclick={() => applyChip(chip.key)}
        aria-pressed={statusFilter === chip.key}
        class="focus-ring flex h-8 items-center gap-1.5 rounded-lg px-3 text-base
          {statusFilter === chip.key
          ? 'bg-ink-900 font-medium text-white'
          : 'bg-white text-ink-700 ring-1 ring-inset ring-ink-200 hover:bg-ink-50'}"
      >
        {chip.label}
        <span class="num text-sm {statusFilter === chip.key ? 'text-ink-300' : 'text-ink-400'}">
          {chip.count}
        </span>
      </button>
    {/each}

    {#if selected.size > 0 && canManage}
      <div class="ml-auto flex items-center gap-2">
        <span class="num text-sm text-ink-500">{selected.size} dipilih</span>
        <Button variant="primary" icon="mail" loading={sending} onclick={sendSelected}>
          Kirim tagihan
        </Button>
      </div>
    {/if}
  </div>

  <Card padded={false}>
    {#if loading}
      <div class="px-4 py-3"><TableSkeleton rows={10} cols={6} /></div>
    {:else if rows.length === 0}
      <div class="flex flex-col items-center gap-2 px-4 py-16 text-center">
        <Icon name="receipt" size={26} class="text-ink-300" />
        <div class="text-base font-medium text-ink-700">Tidak ada tagihan</div>
        <div class="text-sm text-ink-500">Coba pilih filter status yang lain.</div>
      </div>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full border-collapse text-base">
          <thead>
            <tr class="border-b border-ink-200 bg-ink-50">
              {#if canManage}
                <th class="w-10 px-3 py-1 text-left">
                  <!-- Kotak visual 16px, area klik 24px lewat padding label:
                       WCAG 2.5.8 minta target >= 24x24, checkbox native cuma 16. -->
                  <label class="inline-flex size-6 cursor-pointer items-center justify-center">
                    <input
                      type="checkbox"
                      checked={selected.size === rows.length && rows.length > 0}
                      onchange={toggleAll}
                      aria-label="Pilih semua di halaman ini"
                      class="size-4 rounded border-ink-300"
                    />
                  </label>
                </th>
              {/if}
              <th class="px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase"
                >Invoice</th
              >
              <th
                class="hidden px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase lg:table-cell"
                >Keterangan</th
              >
              <th class="px-4 py-2 text-right text-xs font-semibold text-ink-500 uppercase"
                >Jumlah</th
              >
              <th class="px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase">Status</th
              >
              <th
                class="hidden px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase md:table-cell"
                >Jatuh tempo</th
              >
              <th class="px-4 py-2 text-right text-xs font-semibold text-ink-500 uppercase">Aksi</th>
            </tr>
          </thead>
          <tbody>
            {#each rows as inv (inv.id)}
              <tr class="border-b border-ink-100 last:border-0 hover:bg-ink-50">
                {#if canManage}
                  <td class="px-3 py-1.5">
                    <label class="inline-flex size-6 cursor-pointer items-center justify-center">
                      <input
                        type="checkbox"
                        checked={selected.has(inv.id)}
                        onchange={() => toggle(inv.id)}
                        aria-label="Pilih {inv.invoice_number}"
                        class="size-4 rounded border-ink-300"
                      />
                    </label>
                  </td>
                {/if}
                <td class="num px-4 py-2.5 font-medium text-ink-900">{inv.invoice_number}</td>
                <td class="hidden max-w-xs truncate px-4 py-2.5 text-ink-700 lg:table-cell">
                  {inv.description || '—'}
                </td>
                <td class="num px-4 py-2.5 text-right text-ink-900">{formatRupiah(inv.amount)}</td>
                <td class="px-4 py-2.5">
                  <div class="flex items-center gap-1.5">
                    <Badge status={inv.status} />
                    {#if isOverdue(inv)}
                      <!-- Umur tunggakan lebih berguna daripada label "lewat"
                           telanjang: 474 dari 476 piutang di sini sudah >90 hari. -->
                      <Badge tone="negative" label="{daysLate(inv)} hari" />
                    {/if}
                  </div>
                </td>
                <td class="hidden px-4 py-2.5 text-ink-700 md:table-cell">
                  {formatDate(inv.due_date)}
                </td>
                <td class="px-4 py-2.5">
                  <RowActions
                    primary={{
                      label: 'Detail',
                      icon: 'chevronRight',
                      onclick: () => goto(`/v2/admin/invoices/${inv.id}`),
                    }}
                    rest={canManage
                      ? [
                          { label: 'Halaman bayar', icon: 'chevronRight', onclick: () => window.open(`/pay/${inv.id}`, '_blank') },
                          { label: 'Verifikasi bayar', icon: 'check' },
                          { label: 'Kirim ulang', icon: 'mail' },
                        ]
                      : []}
                  />
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <div
        class="flex flex-wrap items-center justify-between gap-2 border-t border-ink-200 bg-ink-50 px-4 py-2"
      >
        <div class="num text-sm text-ink-500">{from}–{to} dari {total} tagihan</div>
        <div class="flex items-center gap-1.5">
          <Button
            size="sm"
            icon="chevronLeft"
            label="Halaman sebelumnya"
            disabled={page <= 1}
            onclick={() => {
              page -= 1;
              load();
            }}
          />
          <span class="num text-sm text-ink-500">Hal {page} / {lastPage}</span>
          <Button
            size="sm"
            icon="chevronRight"
            label="Halaman berikutnya"
            disabled={page >= lastPage}
            onclick={() => {
              page += 1;
              load();
            }}
          />
        </div>
      </div>
    {/if}
  </Card>
</AppShell>
