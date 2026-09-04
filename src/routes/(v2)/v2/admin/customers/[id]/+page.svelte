<script lang="ts">
  /*
    Detail pelanggan v2 — arketipe terberat (versi lama: 3.852 baris,
    21.073 karakter CSS scoped, 8 tab, hero 90 baris).

    Yang berubah secara struktural, bukan kosmetik:

    1. TAGIHAN DIAMBIL LENGKAP. Versi lama memanggil
       `listCustomerPackageInvoices()` tanpa argumen, dan backend memakai
       `per_page.unwrap_or(25)` lalu memotongnya di 100 (batas itu kini 1.000,
       lihat src-tauri/src/services/pagination.rs). Jadi yang datang adalah
       25 invoice TERBARU SE-TENANT, lalu difilter client-side ke langganan
       pelanggan yang sedang dibuka. Terukur di DB: 485 invoice paket milik 482
       langganan, sementara 25 terbaru hanya menyentuh 24 langganan — 453
       pelanggan kehilangan riwayat tagihan. Di sini dipakai `fetchAllRows`.

    2. STATUS DARI LANGGANAN, BUKAN `customers.is_active`. Di tenant ini
       548/548 pelanggan is_active=true sementara 542 langganan suspended, jadi
       badge "Aktif" pada versi lama tidak pernah salah dan tidak pernah
       berguna. Badge di sini merangkum status langganan.

    3. TAB DIMUAT SAAT DIBUKA (lazy). Versi lama memuat 8 sumber data;
       di sini hanya tab aktif yang meminta data, dan hasilnya di-cache.
  */
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import DetailHeader from '$lib/components/ds/DetailHeader.svelte';
  import Tabs from '$lib/components/ds/Tabs.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Icon from '$lib/components/ds/Icon.svelte';
  import FieldRow from '$lib/components/ds/FieldRow.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import TableSkeleton from '$lib/components/ds/TableSkeleton.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import { formatRupiah, formatDate, formatRelative } from '$lib/components/ds/format';
  import { fetchAllRows } from '$lib/utils/fetchAllPages';
  import type {
    Customer,
    CustomerLocation,
    CustomerSubscriptionView,
    CustomerPortalUser,
    Invoice,
  } from '$lib/api/types';

  const customerId = $derived($page.params.id ?? '');

  let customer = $state<Customer | null>(null);
  let loading = $state(true);
  let err = $state('');

  let subs = $state<CustomerSubscriptionView[]>([]);
  let invoices = $state<Invoice[]>([]);
  let locations = $state<CustomerLocation[]>([]);
  let portalUsers = $state<CustomerPortalUser[]>([]);

  /* Cache per tab: kunci = id tab yang sudah selesai dimuat. Mencegah
     permintaan ulang tiap kali pengguna bolak-balik antar tab. */
  let loaded = $state<Record<string, boolean>>({});
  let loadingTab = $state(false);

  let active = $state('ringkasan');

  const canManage = $derived($can('manage', 'customers'));
  /* 'payments' bukan resource yang ada; pakai 'billing' seperti halaman legacy, dan
     terima 'manage' sebagai implikasi 'read' (manage tanpa read tidak masuk akal). */
  const canReadBilling = $derived($can('read', 'billing') || $can('manage', 'billing'));

  const activeSubs = $derived(subs.filter((s) => s.status === 'active'));
  const pendingInstall = $derived(subs.filter((s) => s.status === 'pending_installation'));

  /* Status yang ditampilkan diturunkan dari langganan — lihat catatan 2 di atas. */
  const derivedStatus = $derived.by(() => {
    if (subs.length === 0) return { label: 'Belum ada layanan', tone: 'neutral' as const };
    if (activeSubs.length > 0)
      return { label: `${activeSubs.length} layanan aktif`, tone: 'positive' as const };
    if (pendingInstall.length > 0)
      return { label: 'Menunggu instalasi', tone: 'warning' as const };
    return { label: 'Semua layanan nonaktif', tone: 'negative' as const };
  });

  const unpaid = $derived(
    invoices.filter((i) => i.status === 'pending' || i.status === 'verification_pending'),
  );
  const unpaidTotal = $derived(unpaid.reduce((sum, i) => sum + (i.amount ?? 0), 0));
  const overdue = $derived(
    unpaid.filter((i) => i.due_date && new Date(i.due_date).getTime() < Date.now()),
  );

  const monthlyValue = $derived(activeSubs.reduce((sum, s) => sum + (s.price ?? 0), 0));

  const tabs = $derived(
    [
      { id: 'ringkasan', label: 'Ringkasan' },
      canReadBilling ? { id: 'tagihan', label: 'Tagihan', count: invoices.length || null } : null,
      { id: 'layanan', label: 'Layanan', count: subs.length || null },
      { id: 'lokasi', label: 'Lokasi', count: locations.length || null },
      { id: 'portal', label: 'Akun portal', count: portalUsers.length || null },
    ].filter((t): t is { id: string; label: string; count?: number | null } => t !== null),
  );

  async function ensureTab(id: string) {
    if (loaded[id] || !customerId) return;
    loadingTab = true;
    try {
      if (id === 'tagihan') {
        /* fetchAllRows, BUKAN listCustomerPackageInvoices() telanjang. */
        const all = await fetchAllRows<Invoice>((p, per_page) =>
          api.payment.listCustomerPackageInvoices({ page: p, per_page }),
        );
        const subIds = new Set(subs.map((s) => s.id));
        invoices = all
          .filter((i) => {
            const ext = i.external_id ?? '';
            if (!ext.startsWith('pkgsub:')) return false;
            return subIds.has(ext.split(':')[1] ?? '');
          })
          .sort(
            (a, b) =>
              new Date(b.created_at ?? b.due_date ?? 0).getTime() -
              new Date(a.created_at ?? a.due_date ?? 0).getTime(),
          );
      } else if (id === 'lokasi') {
        locations = await api.customers.locations.list(customerId);
      } else if (id === 'portal') {
        portalUsers = await api.customers.portalUsers.list(customerId);
      }
      loaded = { ...loaded, [id]: true };
    } catch (e) {
      console.warn(`gagal memuat tab ${id}`, e);
    } finally {
      loadingTab = false;
    }
  }

  function selectTab(id: string) {
    active = id;
    void ensureTab(id);
  }

  onMount(async () => {
    if (!customerId) {
      err = 'Id pelanggan tidak ada di URL';
      loading = false;
      return;
    }
    try {
      /* Langganan selalu dimuat di awal: dipakai badge status, StatTile, dan
         penyaring invoice — bukan hanya isi tab Layanan. */
      const [c, subsRes] = await Promise.all([
        api.customers.get(customerId),
        api.customers.subscriptions
          .list(customerId, { page: 1, per_page: 100 })
          .then((r) => r.data ?? [])
          .catch(() => [] as CustomerSubscriptionView[]),
      ]);
      customer = c;
      subs = subsRes;
      loaded = { ringkasan: true, layanan: true };
    } catch (e) {
      err = 'Gagal memuat data pelanggan';
      console.warn('get customer gagal', e);
    } finally {
      loading = false;
    }
  });
</script>

<AppShell title="Detail pelanggan">
  {#if loading}
    <div class="space-y-4">
      <div class="h-10 w-64 animate-pulse rounded-lg bg-ink-100"></div>
      <Card><TableSkeleton rows={6} cols={2} /></Card>
    </div>
  {:else if err || !customer}
    <div
      role="alert"
      class="rounded-lg border border-red-200 bg-red-50 px-3.5 py-2.5 text-base text-red-800"
    >
      {err || 'Pelanggan tidak ditemukan'}
    </div>
  {:else}
    <DetailHeader
      title={customer.name}
      subtitle={customer.customer_number || customer.id}
      statusTone={derivedStatus.tone}
      statusLabel={derivedStatus.label}
      backHref="/v2/admin/customers"
      backLabel="Daftar pelanggan"
      meta={[
        { label: 'Email', value: customer.email || '—' },
        { label: 'Telepon', value: customer.phone || '—' },
        { label: 'Nilai bulanan', value: formatRupiah(monthlyValue) },
        { label: 'Diperbarui', value: formatRelative(customer.updated_at) },
      ]}
    >
      {#snippet actions()}
        {#if canManage}
          <Button variant="primary" icon="plus">Tambah layanan</Button>
          <RowActions
            primary={{ label: 'Ubah data', icon: 'cog' }}
            rest={[
              { label: 'Buat tagihan', icon: 'receipt' },
              { label: 'Kirim WhatsApp', icon: 'inbox' },
              { label: 'Kirim email', icon: 'mail' },
              { label: 'Suspend layanan', icon: 'alert' },
              { label: 'Hapus pelanggan', icon: 'close', danger: true },
            ]}
          />
        {/if}
      {/snippet}
    </DetailHeader>

    <Tabs items={tabs} {active} panelId="detail-panel" onselect={selectTab} />

    <div id="detail-panel" role="tabpanel">
      {#if active === 'ringkasan'}
        <div class="grid gap-4 lg:grid-cols-3">
          <Card class="lg:col-span-2" title="Data pelanggan">
            <dl class="grid gap-x-8 sm:grid-cols-2">
              <FieldRow label="Nama" value={customer.name} />
              <FieldRow label="Nomor pelanggan" value={customer.customer_number} mono />
              <FieldRow label="Email" value={customer.email} />
              <FieldRow label="Telepon" value={customer.phone} mono />
              <FieldRow label="Terdaftar" value={formatDate(customer.created_at)} />
              <FieldRow label="Diperbarui" value={formatDate(customer.updated_at, true)} />
              <div class="sm:col-span-2">
                <FieldRow label="Catatan" value={customer.notes} />
              </div>
            </dl>
          </Card>

          <div class="space-y-4">
            <Card title="Posisi keuangan">
              <div class="space-y-4">
                <StatTile
                  label="Belum dibayar"
                  value={formatRupiah(unpaidTotal)}
                  hint={canReadBilling && loaded.tagihan
                    ? `${unpaid.length} invoice · ${overdue.length} jatuh tempo`
                    : 'buka tab Tagihan untuk memuat'}
                  tone={unpaidTotal > 0 ? 'negative' : 'positive'}
                />
                <StatTile
                  label="Nilai bulanan"
                  value={formatRupiah(monthlyValue)}
                  hint="{activeSubs.length} dari {subs.length} langganan aktif"
                />
              </div>
            </Card>

            <Card title="Layanan">
              {#if subs.length === 0}
                <p class="text-base text-ink-500">Belum ada langganan.</p>
              {:else}
                <ul class="space-y-2.5">
                  {#each subs.slice(0, 4) as s (s.id)}
                    <li class="flex items-center justify-between gap-3">
                      <div class="min-w-0">
                        <div class="truncate text-base text-ink-800">
                          {s.package_name || 'Paket tanpa nama'}
                        </div>
                        <div class="num text-sm text-ink-400">{formatRupiah(s.price)}/bln</div>
                      </div>
                      <Badge status={s.status} />
                    </li>
                  {/each}
                </ul>
                {#if subs.length > 4}
                  <button
                    onclick={() => selectTab('layanan')}
                    class="focus-ring mt-3 rounded text-sm text-brand-700 hover:underline"
                  >
                    Lihat {subs.length - 4} layanan lainnya
                  </button>
                {/if}
              {/if}
            </Card>
          </div>
        </div>
      {:else if active === 'tagihan'}
        <Card padded={false}>
          {#if loadingTab}
            <div class="px-4 py-3"><TableSkeleton rows={8} cols={4} /></div>
          {:else if invoices.length === 0}
            <div class="flex flex-col items-center gap-2 px-4 py-14 text-center">
              <Icon name="receipt" size={26} class="text-ink-300" />
              <div class="text-base font-medium text-ink-700">Belum ada tagihan</div>
              <div class="text-sm text-ink-500">
                Tagihan muncul setelah langganan pelanggan ini ditagihkan.
              </div>
            </div>
          {:else}
            <div class="overflow-x-auto">
              <table class="w-full border-collapse text-base">
                <thead>
                  <tr class="border-b border-ink-200 bg-ink-50">
                    <th class="px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase"
                      >Invoice</th
                    >
                    <th class="px-4 py-2 text-right text-xs font-semibold text-ink-500 uppercase"
                      >Jumlah</th
                    >
                    <th class="px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase"
                      >Status</th
                    >
                    <th
                      class="hidden px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase md:table-cell"
                      >Jatuh tempo</th
                    >
                    <th class="px-4 py-2 text-right text-xs font-semibold text-ink-500 uppercase"
                      >Aksi</th
                    >
                  </tr>
                </thead>
                <tbody>
                  {#each invoices as inv (inv.id)}
                    <tr class="border-b border-ink-100 last:border-0 hover:bg-ink-50">
                      <td class="num px-4 py-2.5 text-ink-900">{inv.invoice_number}</td>
                      <td class="num px-4 py-2.5 text-right text-ink-900"
                        >{formatRupiah(inv.amount)}</td
                      >
                      <td class="px-4 py-2.5"><Badge status={inv.status} /></td>
                      <td class="hidden px-4 py-2.5 text-ink-700 md:table-cell">
                        {formatDate(inv.due_date)}
                      </td>
                      <td class="px-4 py-2.5">
                        <RowActions
                          primary={{
                            label: 'Buka',
                            icon: 'chevronRight',
                            onclick: () => window.open(`/pay/${inv.id}`, '_blank'),
                          }}
                          rest={canManage ? [{ label: 'Verifikasi bayar', icon: 'check' }] : []}
                        />
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
            <div class="border-t border-ink-200 bg-ink-50 px-4 py-2 text-sm text-ink-500">
              {invoices.length} tagihan · belum dibayar {formatRupiah(unpaidTotal)}
              {#if overdue.length > 0}· {overdue.length} jatuh tempo{/if}
            </div>
          {/if}
        </Card>
      {:else if active === 'layanan'}
        <div class="grid gap-4 md:grid-cols-2">
          {#each subs as s (s.id)}
            <Card>
              <div class="mb-3 flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="truncate text-base font-medium text-ink-900">
                    {s.package_name || 'Paket tanpa nama'}
                  </div>
                  <div class="num text-sm text-ink-400">{formatRupiah(s.price)}/bln</div>
                </div>
                <Badge status={s.status} />
              </div>
              <dl class="grid grid-cols-2 gap-x-6">
                <FieldRow label="Lokasi" value={s.location_label} />
                <FieldRow label="Router" value={s.router_name} />
                <FieldRow label="Mulai" value={formatDate(s.starts_at)} />
                <FieldRow label="Siklus" value={s.billing_cycle} />
              </dl>
            </Card>
          {:else}
            <Card class="md:col-span-2">
              <p class="text-base text-ink-500">Pelanggan ini belum punya langganan.</p>
            </Card>
          {/each}
        </div>
      {:else if active === 'lokasi'}
        {#if loadingTab}
          <Card><TableSkeleton rows={4} cols={2} /></Card>
        {:else}
          <div class="grid gap-4 md:grid-cols-2">
            {#each locations as loc (loc.id)}
              <Card>
                <div class="mb-2 flex items-center gap-2">
                  <Icon name="pin" size={15} class="text-ink-400" />
                  <span class="text-base font-medium text-ink-900">{loc.label}</span>
                </div>
                <dl>
                  <FieldRow
                    label="Alamat"
                    value={[loc.address_line1, loc.address_line2, loc.city, loc.state]
                      .filter(Boolean)
                      .join(', ')}
                  />
                  <FieldRow
                    label="Koordinat"
                    value={loc.latitude != null && loc.longitude != null
                      ? `${loc.latitude}, ${loc.longitude}`
                      : null}
                    mono
                  />
                </dl>
              </Card>
            {:else}
              <Card class="md:col-span-2">
                <p class="text-base text-ink-500">Belum ada lokasi terdaftar.</p>
              </Card>
            {/each}
          </div>
        {/if}
      {:else if active === 'portal'}
        {#if loadingTab}
          <Card><TableSkeleton rows={3} cols={2} /></Card>
        {:else}
          <Card padded={false}>
            {#if portalUsers.length === 0}
              <div class="flex flex-col items-center gap-2 px-4 py-14 text-center">
                <Icon name="users" size={26} class="text-ink-300" />
                <div class="text-base font-medium text-ink-700">Belum ada akun portal</div>
                <div class="text-sm text-ink-500">
                  Pelanggan ini belum bisa masuk ke portal mandiri.
                </div>
              </div>
            {:else}
              <table class="w-full border-collapse text-base">
                <thead>
                  <tr class="border-b border-ink-200 bg-ink-50">
                    <th class="px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase"
                      >Nama</th
                    >
                    <th class="px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase"
                      >Email</th
                    >
                    <th class="px-4 py-2 text-left text-xs font-semibold text-ink-500 uppercase"
                      >Dibuat</th
                    >
                  </tr>
                </thead>
                <tbody>
                  {#each portalUsers as u (u.customer_user_id)}
                    <tr class="border-b border-ink-100 last:border-0">
                      <td class="px-4 py-2.5 text-ink-900">{u.name}</td>
                      <td class="px-4 py-2.5 text-ink-700">{u.email}</td>
                      <td class="px-4 py-2.5 text-ink-700">{formatDate(u.created_at)}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </Card>
        {/if}
      {/if}
    </div>
  {/if}
</AppShell>
