<script lang="ts">
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
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import TableSkeleton from '$lib/components/ds/TableSkeleton.svelte';
  import { formatRelative } from '$lib/components/ds/format';
  import type { CustomerListItem } from '$lib/api/types';

  let rows = $state<CustomerListItem[]>([]);
  let total = $state(0);
  let page = $state(1);
  let perPage = $state(25);
  let q = $state('');
  let statusFilter = $state<'all' | 'active' | 'inactive'>('all');
  let serviceFilter = $state<'all' | 'active' | 'inactive' | 'none'>('all');
  let loading = $state(true);
  let err = $state('');

  /* Hitungan per filter LAYANAN, bukan per `customers.is_active`.
     Alasan: di tenant ini 548/548 pelanggan is_active=true sementara 542
     langganan berstatus suspended, jadi chip "Aktif 548" tidak membedakan
     apa pun. Yang menentukan pendapatan adalah status langganan. */
  let counts = $state({ all: 0, svcActive: 0, svcInactive: 0, svcNone: 0, pending: 0 });

  const canManage = $derived($can('manage', 'customers'));
  const lastPage = $derived(Math.max(1, Math.ceil(total / perPage)));
  const from = $derived(total === 0 ? 0 : (page - 1) * perPage + 1);
  const to = $derived(Math.min(page * perPage, total));

  /* Chip filter cepat. Menggantikan 3 dropdown terpisah di halaman lama:
     pilihan yang sering dipakai jadi satu klik, dan jumlahnya terlihat. */
  const chips = $derived([
    { key: 'all', label: 'Semua', count: counts.all },
    { key: 'svc-active', label: 'Layanan aktif', count: counts.svcActive },
    { key: 'svc-inactive', label: 'Layanan nonaktif', count: counts.svcInactive },
    { key: 'pending', label: 'Menunggu instalasi', count: counts.pending },
    { key: 'svc-none', label: 'Tanpa layanan', count: counts.svcNone },
  ]);

  let activeChip = $state('all');

  function applyChip(key: string) {
    activeChip = key;
    page = 1;

    statusFilter = 'all';
    serviceFilter =
      key === 'svc-active'
        ? 'active'
        : key === 'svc-inactive'
          ? 'inactive'
          : key === 'svc-none'
            ? 'none'
            : 'all';

    load();
  }

  /* Placeholder hasil impor MixRadius: baris ini bukan pelanggan nyata,
     jadi aksinya disembunyikan supaya tidak ada yang mengirim WA ke sana. */
  function isPlaceholder(c: CustomerListItem): boolean {
    return /unassigned|system import/i.test(c.name ?? '');
  }

  function serviceLabel(c: CustomerListItem): string {
    if (c.pending_installations > 0) return 'Menunggu instalasi';
    if (c.active_subscriptions > 0) return `${c.active_subscriptions} layanan aktif`;
    if (c.subscription_count > 0) return 'Layanan nonaktif';
    return 'Belum ada layanan';
  }

  function serviceTone(c: CustomerListItem) {
    if (c.pending_installations > 0) return 'warning' as const;
    if (c.active_subscriptions > 0) return 'positive' as const;
    if (c.subscription_count > 0) return 'negative' as const;
    return 'neutral' as const;
  }

  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  function onSearch(value: string) {
    q = value;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      page = 1;
      load();
    }, 300);
  }

  async function load() {
    loading = true;
    err = '';

    try {
      const res = await api.customers.list({
        q: q || undefined,
        page,
        perPage,
        status: statusFilter,
        service: serviceFilter,
        installation: activeChip === 'pending' ? 'pending' : 'all',
      });
      rows = res.data ?? [];
      total = res.total ?? 0;
    } catch (e) {
      err = 'Gagal memuat daftar pelanggan';
      console.warn('list customers gagal', e);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    /* Hitungan chip diambil dari endpoint list dengan per_page=1: yang dipakai
       hanya `total`, jadi biaya transfernya satu baris per chip. Ini jauh lebih
       murah daripada menarik 548 baris hanya untuk dihitung di klien. */
    const countOf = (service: 'all' | 'active' | 'inactive' | 'none', installation?: 'pending') =>
      api.customers
        .list({ page: 1, perPage: 1, service, installation: installation ?? 'all' })
        .then((r) => r.total ?? 0)
        .catch(() => 0);

    await Promise.all([
      load(),
      Promise.all([
        countOf('all'),
        countOf('active'),
        countOf('inactive'),
        countOf('none'),
        countOf('all', 'pending'),
      ]).then(([all, svcActive, svcInactive, svcNone, pending]) => {
        counts = { all, svcActive, svcInactive, svcNone, pending };
      }),
    ]);
  });
</script>

<AppShell title="Pelanggan">
  <PageHeader
    title="Pelanggan"
    desc="{counts.all} terdaftar · {counts.svcActive} punya layanan aktif · {counts.pending} menunggu instalasi"
  >
    {#snippet actions()}
      <Button icon="download">Ekspor</Button>
      {#if canManage}
        <Button variant="primary" icon="plus">Tambah pelanggan</Button>
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

  <!-- Filter: chip + satu kotak cari. -->
  <div class="mb-3 flex flex-wrap items-center gap-2">
    {#each chips as chip}
      <button
        onclick={() => applyChip(chip.key)}
        aria-pressed={activeChip === chip.key}
        class="focus-ring flex h-8 items-center gap-1.5 rounded-lg px-3 text-base
          {activeChip === chip.key
          ? 'bg-ink-900 font-medium text-white'
          : 'bg-white text-ink-700 ring-1 ring-inset ring-ink-200 hover:bg-ink-50'}"
      >
        {chip.label}
        <span
          class="num text-sm {activeChip === chip.key ? 'text-ink-300' : 'text-ink-400'}"
        >
          {chip.count}
        </span>
      </button>
    {/each}

    <div class="relative ml-auto w-full sm:w-64">
      <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400">
        <Icon name="search" size={15} />
      </span>
      <input
        value={q}
        oninput={(e) => onSearch((e.currentTarget as HTMLInputElement).value)}
        type="search"
        placeholder="Cari nama, email, nomor"
        aria-label="Cari pelanggan"
        class="h-8 w-full rounded-lg bg-white pr-3 pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400 focus:ring-brand-600 focus:outline-none"
      />
    </div>
  </div>

  <Card padded={false}>
    {#if loading}
      <div class="px-4 py-3">
        <TableSkeleton rows={10} cols={5} />
      </div>
    {:else if rows.length === 0}
      <div class="flex flex-col items-center gap-2 px-4 py-16 text-center">
        <Icon name="inbox" size={26} class="text-ink-300" />
        <div class="text-base font-medium text-ink-700">Tidak ada pelanggan cocok</div>
        <div class="text-sm text-ink-500">
          {q ? `Tidak ada hasil untuk "${q}".` : 'Coba ubah filter di atas.'}
        </div>
      </div>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full border-collapse text-base">
          <thead>
            <tr class="border-b border-ink-200 bg-ink-50">
              <th class="px-4 py-2 text-left text-xs font-semibold tracking-wide text-ink-500 uppercase"
                >Pelanggan</th
              >
              <th
                class="hidden px-4 py-2 text-left text-xs font-semibold tracking-wide text-ink-500 uppercase lg:table-cell"
                >Kontak</th
              >
              <th class="px-4 py-2 text-left text-xs font-semibold tracking-wide text-ink-500 uppercase"
                >Layanan</th
              >
              <th
                class="hidden px-4 py-2 text-left text-xs font-semibold tracking-wide text-ink-500 uppercase md:table-cell"
                >Diperbarui</th
              >
              <th class="px-4 py-2 text-right text-xs font-semibold tracking-wide text-ink-500 uppercase"
                >Aksi</th
              >
            </tr>
          </thead>
          <tbody>
            {#each rows as c (c.id)}
              <tr class="border-b border-ink-100 last:border-0 hover:bg-ink-50">
                <td class="px-4 py-2.5">
                  <div class="flex items-center gap-1.5">
                    <span class="font-medium text-ink-900">{c.name}</span>
                    {#if !c.is_active}
                      <Badge tone="negative" label="Nonaktif" />
                    {/if}
                  </div>
                  <div class="num text-sm text-ink-400">{c.customer_number || '—'}</div>
                </td>
                <td class="hidden px-4 py-2.5 lg:table-cell">
                  <div class="text-ink-700">{c.email || '—'}</div>
                  <div class="num text-sm text-ink-400">{c.phone || '—'}</div>
                </td>
                <td class="px-4 py-2.5">
                  <Badge tone={serviceTone(c)} label={serviceLabel(c)} />
                </td>
                <td class="hidden px-4 py-2.5 text-sm text-ink-500 md:table-cell">
                  {formatRelative(c.updated_at)}
                </td>
                <td class="px-4 py-2.5">
                  {#if isPlaceholder(c)}
                    <div class="text-right text-sm text-ink-400">—</div>
                  {:else}
                    <RowActions
                      primary={{
                        label: 'Buka',
                        icon: 'chevronRight',
                        onclick: () => goto(`/v2/admin/customers/${c.id}`),
                      }}
                      rest={canManage
                        ? [
                            { label: 'Tambah layanan', icon: 'wifi' },
                            { label: 'Buat tagihan', icon: 'receipt' },
                            { label: 'Kirim WhatsApp', icon: 'inbox' },
                            { label: 'Kirim email', icon: 'mail' },
                            { label: 'Hapus pelanggan', icon: 'close', danger: true },
                          ]
                        : []}
                    />
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <div
        class="flex flex-wrap items-center justify-between gap-2 border-t border-ink-200 bg-ink-50 px-4 py-2"
      >
        <div class="num text-sm text-ink-500">
          {from}–{to} dari {total} pelanggan
        </div>
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
