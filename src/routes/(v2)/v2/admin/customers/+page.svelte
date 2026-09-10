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
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import { formatRelative } from '$lib/components/ds/format';
  import type { Column } from '$lib/components/ds/table-types';
  import type { CustomerListItem } from '$lib/api/types';

  import { t } from 'svelte-i18n';
  import { get as getStore } from 'svelte/store';
  const columns = $derived<Column[]>([
    { key: 'name', label: $t('admin.customers.columns.customer') },
    { key: 'contact', label: $t('admin.customers.columns.contact'), hideSm: true },
    { key: 'services', label: $t('admin.customers.columns.service') },
    { key: 'updated', label: $t('admin.customers.columns.updated'), hideSm: true },
    { key: 'actions', label: $t('common.actions'), align: 'right', width: '120px' },
  ]);

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

  /* Chip filter cepat. Menggantikan 3 dropdown terpisah di halaman lama:
     pilihan yang sering dipakai jadi satu klik, dan jumlahnya terlihat. */
  const chips = $derived([
    { key: 'all', label: $t('common.all'), count: counts.all },
    { key: 'svc-active', label: $t('admin.customers.list_v2.chip_active'), count: counts.svcActive },
    { key: 'svc-inactive', label: $t('admin.customers.list_v2.chip_inactive'), count: counts.svcInactive },
    { key: 'pending', label: $t('admin.customers.list_v2.chip_pending'), count: counts.pending },
    { key: 'svc-none', label: $t('admin.customers.list_v2.chip_none'), count: counts.svcNone },
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
    const tt = getStore(t);
    if (c.pending_installations > 0) return tt('admin.customers.list_v2.chip_pending');
    if (c.active_subscriptions > 0) return tt('admin.customers.list_v2.n_active', { values: { n: c.active_subscriptions } });
    if (c.subscription_count > 0) return tt('admin.customers.list_v2.chip_inactive');
    return tt('admin.customers.list_v2.no_service');
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
        .catch((e) => {
          console.error('hitung chip pelanggan gagal:', e);
          return 0;
        });

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

<AppShell title={ $t('admin.customers.title') }>
  <PageHeader
    title={ $t('admin.customers.title') }
    desc={ $t('admin.customers.list_v2.desc', { values: { a: counts.all, b: counts.svcActive, c: counts.pending } }) }
  >
    {#snippet actions()}
      <Button icon="download">{ $t('admin.customers.list_v2.export') }</Button>
      {#if canManage}
        <Button variant="primary" icon="plus">{ $t('admin.customers.list_v2.add') }</Button>
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
        placeholder={ $t('admin.customers.list_v2.search_ph') }
        aria-label={ $t('admin.customers.list_v2.search_aria') }
        class="h-8 w-full rounded-lg bg-white pr-3 pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400 focus:ring-brand-600 focus:outline-none"
      />
    </div>
  </div>

  <Card padded={false}>
    <DataTable
      {columns}
      rows={rows}
      {loading}
      pageSize={perPage}
      page={page}
      {total}
      onpage={(p) => {
        page = p;
        load();
      }}
      emptyTitle={ $t('admin.customers.list_v2.empty_title') }
      emptyHint={q ? $t('common.no_results_for', { values: { q } }) : $t('admin.customers.list_v2.empty_hint')}
      footNote={ $t('admin.customers.list_v2.foot', { values: { n: total } }) }
    >
      {#snippet cell(c: CustomerListItem, col: Column)}
        {#if col.key === 'name'}
          <div class="flex items-center gap-1.5">
            <span class="font-medium text-ink-900">{c.name}</span>
            {#if !c.is_active}
              <Badge tone="negative" label={ $t('admin.customers.list_v2.disabled') } />
            {/if}
          </div>
          <div class="num text-sm text-ink-400">{c.customer_number || '—'}</div>
        {:else if col.key === 'contact'}
          <div class="text-ink-700">{c.email || '—'}</div>
          <div class="num text-sm text-ink-400">{c.phone || '—'}</div>
        {:else if col.key === 'services'}
          <Badge tone={serviceTone(c)} label={serviceLabel(c)} />
        {:else if col.key === 'updated'}
          <span class="text-sm text-ink-500">{formatRelative(c.updated_at)}</span>
        {:else if col.key === 'actions'}
          {#if isPlaceholder(c)}
            <div class="text-right text-sm text-ink-400">—</div>
          {:else}
            <RowActions
              primary={{
                label: $t('common.open'),
                icon: 'chevronRight',
                onclick: () => goto(`/v2/admin/customers/${c.id}`),
              }}
              rest={canManage
                ? [
                    { label: $t('admin.customers.list_v2.act_service'), icon: 'wifi' },
                    { label: $t('admin.customers.list_v2.act_invoice'), icon: 'receipt' },
                    { label: $t('admin.customers.list_v2.act_wa'), icon: 'inbox' },
                    { label: $t('admin.customers.list_v2.act_email'), icon: 'mail' },
                    { label: $t('admin.customers.list_v2.act_delete'), icon: 'close', danger: true },
                  ]
                : []}
            />
          {/if}
        {/if}
      {/snippet}
    </DataTable>
  </Card>
</AppShell>
