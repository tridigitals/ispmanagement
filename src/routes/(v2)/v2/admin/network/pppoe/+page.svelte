<script lang="ts">
  /*
    PPPoE v2 — halaman jaringan. Versi lama: 1.434 baris + 6.468 karakter CSS.

    Temuan DB yang membentuk desain halaman ini (1.037 akun terukur):

    1. `is_provisioned` BUKAN indikator kesehatan. Nilainya berkorelasi 100%
       dengan `account_source`: 545 akun managed_radius semuanya true, 492 akun
       router semuanya false. Jadi filter "belum di-apply 492" pada versi lama
       menakut-nakuti tanpa sebab — itu cuma cara akun dibuat, bukan masalah.

    2. YANG BENAR-BENAR MASALAH: 7 akun bersumber router yang TIDAK ADA di
       router (`router_present=false`, tidak disabled). Ini drift nyata antara
       aplikasi dan perangkat, dan versi lama tidak pernah menyorotinya.

    3. 542 akun managed_radius berstatus disabled — jumlahnya sama dengan 542
       langganan suspended. Itu isolir massal hasil impor MixRadius, bukan
       kerusakan.

    Karena itu tile dan chip di sini memakai tiga status operasional
    (melayani / terisolir / hilang di router), bukan `is_provisioned`.
  */
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { can } from '$lib/stores/auth';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import Card from '$lib/components/ds/Card.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Icon from '$lib/components/ds/Icon.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import type { Column } from '$lib/components/ds/table-types';
  import AttentionPanel from '$lib/components/ds/AttentionPanel.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { toast } from '$lib/stores/toast';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { fetchAllPages, fetchAllRows } from '$lib/utils/fetchAllPages';
  import type {
    PppoeAccountPublic,
    CustomerListItem,
    IspPackageRouterMappingView,
  } from '$lib/api/types';
  import { getPppoeAssignmentPayload } from '$lib/utils/pppoePackageAssignment';
  import { createThenApplyPppoeAccount, PppoeCreateApplyError } from '$lib/utils/pppoeCreateProvisioning';
  import { loadPppoeAccountModal } from '../../../../../(app)/admin/network/pppoe/pppoePageModules';

  import { t } from 'svelte-i18n';
  /* api.mikrotik.routers.list() masih bertipe Promise<any[]> di lib/api/mikrotik.ts;
     dua field yang dipakai halaman ini dinyatakan eksplisit di sini. */
  type RouterRef = { id: string; name: string };

  type Health = 'serving' | 'isolated' | 'missing' | 'draft';
  type ChipKey = 'all' | Health;

  let all = $state<PppoeAccountPublic[]>([]);
  let complete = $state(true);
  let customerName = $state<Map<string, string>>(new Map());
  let routerName = $state<Map<string, string>>(new Map());
  let loading = $state(true);
  let err = $state('');

  let q = $state('');
  let chip = $state<ChipKey>('all');
  /* ── A-15: create/edit akun PPPoE di v2 — reuse modal legacy (presentational)
     yang sudah dipakai halaman (app) lewat loader cache-nya. ──────────────── */
  type LocationRow = { id: string; label: string };

  let showCreate = $state(false);
  let showEdit = $state(false);
  let saving = $state(false);
  let editRow = $state<PppoeAccountPublic | null>(null);
  let PppoeAccountModalComponent = $state<any | null>(null);

  let formRouterId = $state('');
  let formCustomerId = $state('');
  let formLocationId = $state('');
  let formUsername = $state('');
  let formPassword = $state('');
  let formRouterProfileName = $state('');
  let formRemoteAddress = $state('');
  let formAddressPool = $state('');
  let formDisabled = $state(false);
  let formComment = $state('');
  let formPackageId = $state('');
  let formAccountSource = $state<'router' | 'managed_radius'>('router');

  let locations = $state<LocationRow[]>([]);
  let routerPackageMappings = $state<IspPackageRouterMappingView[]>([]);

  const routerOptions = $derived([...routerName.entries()].map(([id, name]) => ({ label: name, value: id })));
  const customerOptions = $derived([...customerName.entries()].map(([id, name]) => ({ label: name, value: id })));
  const locationOptions = $derived(locations.map((l) => ({ label: l.label, value: l.id })));
  const packageOptions = $derived.by(() => {
    const seen = new Set<string>();
    const out: Array<{ label: string; value: string }> = [];
    for (const m of routerPackageMappings) {
      if (!m?.package_id || seen.has(m.package_id)) continue;
      seen.add(m.package_id);
      out.push({ label: m.package_name, value: m.package_id });
    }
    return out;
  });
  function assignmentPayload() {
    return getPppoeAssignmentPayload({
      packageId: formPackageId,
      mappings: routerPackageMappings,
      current: {
        router_profile_name: formRouterProfileName,
        remote_address: formRemoteAddress,
        address_pool: formAddressPool,
      },
    });
  }
  const packageSelectionHasMissingMapping = $derived(
    Boolean(formPackageId) && !assignmentPayload().hasPackageMapping,
  );

  const sourceLabel = (source: 'router' | 'managed_radius') =>
    source === 'managed_radius' ? 'Managed RADIUS' : 'Secret router';
  const sourceDisabledHintLabel = (source: 'router' | 'managed_radius') =>
    source === 'managed_radius'
      ? 'Nonaktifkan akun di RADIUS terpusat saat Apply ke RADIUS diklik.'
      : 'Nonaktifkan akun PPPoE (diterapkan ke router saat Apply diklik).';
  const sourceCreateActionLabel = (source: 'router' | 'managed_radius') =>
    source === 'managed_radius' ? 'Simpan & terapkan ke RADIUS' : 'Simpan & terapkan ke router';

  async function ensurePppoeAccountModalComponent() {
    if (PppoeAccountModalComponent) return;
    const modules = await loadPppoeAccountModal();
    PppoeAccountModalComponent = modules.PppoeAccountModalComponent;
  }

  async function loadLocations(customerId: string) {
    if (!customerId) {
      locations = [];
      return;
    }
    try {
      const rows: any[] = (await api.customers.locations.list(customerId)) as any;
      locations = (rows || []).map((l) => ({ id: l.id, label: l.label }));
    } catch (e) {
      console.error('locations gagal dimuat:', e);
      locations = [];
    }
  }

  async function loadRouterPackages(routerId: string) {
    if (!routerId) {
      routerPackageMappings = [];
      return;
    }
    try {
      routerPackageMappings = await api.ispPackages.routerMappings.list({ router_id: routerId });
    } catch (e) {
      console.error('mapping paket router gagal:', e);
      routerPackageMappings = [];
    }
  }

  function applyPackageToForm(pkgId: string) {
    const resolved = assignmentPayload();
    formRouterProfileName = resolved.router_profile_name || '';
    formRemoteAddress = resolved.remote_address || '';
    formAddressPool = resolved.address_pool || '';
  }

  function resetForm() {
    formRouterId = '';
    formCustomerId = '';
    formLocationId = '';
    formUsername = '';
    formPassword = '';
    formPackageId = '';
    formRouterProfileName = '';
    formRemoteAddress = '';
    formAddressPool = '';
    formDisabled = false;
    formComment = '';
    formAccountSource = 'router';
    locations = [];
    routerPackageMappings = [];
    editRow = null;
  }

  async function openCreate() {
    if (!canManage) return;
    resetForm();
    await ensurePppoeAccountModalComponent();
    showCreate = true;
  }

  async function openEdit(row: PppoeAccountPublic) {
    if (!canManage) return;
    resetForm();
    editRow = row;
    formRouterId = row.router_id;
    formCustomerId = row.customer_id;
    formLocationId = row.location_id || '';
    formUsername = row.username;
    formPassword = '';
    formPackageId = row.package_id || '';
    formRouterProfileName = row.router_profile_name || '';
    formRemoteAddress = row.remote_address || '';
    formAddressPool = row.address_pool || '';
    formDisabled = Boolean(row.disabled);
    formComment = row.comment || '';
    formAccountSource = row.account_source || 'router';
    await Promise.all([
      loadLocations(row.customer_id),
      loadRouterPackages(row.router_id),
      ensurePppoeAccountModalComponent(),
    ]);
    showEdit = true;
  }

  async function submitCreate() {
    if (saving) return;
    if (packageSelectionHasMissingMapping) {
      toast.error($t('admin.customers.pppoe.v2.t_no_mapping'));
      return;
    }
    if (!formRouterId || !formCustomerId || !formLocationId || !formUsername.trim() || !formPassword) return;
    saving = true;
    try {
      const payload = assignmentPayload();
      const result = await createThenApplyPppoeAccount({
        create: () =>
          api.pppoe.accounts.create({
            router_id: formRouterId,
            customer_id: formCustomerId,
            location_id: formLocationId,
            username: formUsername.trim(),
            password: formPassword,
            package_id: formPackageId || null,
            router_profile_name: payload.router_profile_name,
            remote_address: payload.remote_address,
            address_pool: payload.address_pool,
            disabled: formDisabled,
            comment: formComment.trim() || null,
            account_source: formAccountSource,
          }),
        apply: (id) => api.pppoe.accounts.apply(id),
      });
      toast.success(result.applySucceeded ? $t('admin.customers.pppoe.v2.t_created_applied') : $t('admin.customers.pppoe.v2.t_created'));
      showCreate = false;
      await load();
    } catch (e: unknown) {
      if (e instanceof PppoeCreateApplyError) {
        toast.error(
          $t('admin.customers.pppoe.v2.t_saved_apply_fail') + ' ' + extractApiErrorMessage(e.applyError ?? e),
        );
        showCreate = false;
        await load();
      } else {
        toast.error($t('admin.customers.pppoe.v2.t_create_failed', { values: {message: extractApiErrorMessage(e) }}));
      }
    } finally {
      saving = false;
    }
  }

  async function submitEdit() {
    if (saving || !editRow) return;
    if (!formUsername.trim()) return;
    if (packageSelectionHasMissingMapping) {
      toast.error($t('admin.customers.pppoe.v2.t_no_mapping'));
      return;
    }
    saving = true;
    try {
      const payload = assignmentPayload();
      await api.pppoe.accounts.update(editRow.id, {
        customer_id: formCustomerId || null,
        location_id: formLocationId || null,
        username: formUsername.trim(),
        password: formPassword || undefined,
        package_id: formPackageId || null,
        router_profile_name: payload.router_profile_name,
        remote_address: payload.remote_address,
        address_pool: payload.address_pool,
        disabled: formDisabled,
        comment: formComment.trim() || null,
        account_source: formAccountSource,
      });
      toast.success($t('admin.customers.pppoe.v2.t_updated'));
      showEdit = false;
      await load();
    } catch (e: unknown) {
      toast.error($t('admin.customers.pppoe.v2.t_save_failed', { values: {message: extractApiErrorMessage(e) }}));
    } finally {
      saving = false;
    }
  }

  // P-01: tombol aksi baris di v2 sempat tanpa handler. Di-wire ke API yang
  // sama dengan versi lama; create/edit penuh via modal = scope parity A-15.
  let busyId = $state<string | null>(null);
  let deleteTarget = $state<PppoeAccountPublic | null>(null);
  let showDeleteConfirm = $state(false);
  let deleting = $state(false);
  let routerFilter = $state('');
  let page = $state(1);
  let perPage = $state(25);

  const canManage = $derived($can('manage', 'pppoe'));

  /**
   * Status operasional satu akun. Urutan pemeriksaan penting: "hilang di
   * router" diperiksa sebelum disabled, karena akun yang tidak ada di
   * perangkat adalah masalah konfigurasi terlepas dari flag disabled-nya.
   */
  function healthOf(a: PppoeAccountPublic): Health {
    if (a.account_source === 'router' && !a.router_present) return 'missing';
    if (a.disabled) return 'isolated';
    if (a.account_source === 'managed_radius' && !a.is_provisioned) return 'draft';
    return 'serving';
  }

  const healthMeta = $derived<Record<Health, { label: string; tone: 'positive' | 'negative' | 'warning' | 'neutral' }>>({
    serving: { label: $t('admin.customers.pppoe.v2.stat_serving'), tone: 'positive' },
    isolated: { label: $t('admin.customers.pppoe.v2.stat_isolated'), tone: 'neutral' },
    missing: { label: $t('admin.customers.pppoe.v2.stat_missing'), tone: 'negative' },
    draft: { label: $t('admin.customers.pppoe.v2.stat_draft'), tone: 'warning' },
  });

  const counts = $derived.by(() => {
    const c = { all: all.length, serving: 0, isolated: 0, missing: 0, draft: 0 };
    for (const a of all) c[healthOf(a)]++;
    return c;
  });

  const routers = $derived(
    [...routerName.entries()]
      .map(([id, name]) => ({ id, name, n: all.filter((a) => a.router_id === id).length }))
      .filter((r) => r.n > 0)
      .sort((a, b) => b.n - a.n),
  );

  const filtered = $derived.by(() => {
    const needle = q.trim().toLowerCase();
    return all.filter((a) => {
      if (chip !== 'all' && healthOf(a) !== chip) return false;
      if (routerFilter && a.router_id !== routerFilter) return false;
      if (!needle) return true;
      const name = customerName.get(a.customer_id) ?? '';
      return (
        a.username.toLowerCase().includes(needle) ||
        name.toLowerCase().includes(needle) ||
        (a.comment ?? '').toLowerCase().includes(needle) ||
        (a.remote_address ?? '').toLowerCase().includes(needle)
      );
    });
  });

  const columns: Column[] = [
    { key: 'username', label: 'Username' },
    { key: 'customer', label: 'Pelanggan', hideSm: true },
    { key: 'status', label: 'Status' },
    { key: 'router', label: 'Router', hideSm: true },
    { key: 'ip', label: 'IP / pool', hideSm: true },
    { key: 'actions', label: 'Aksi', align: 'right' },
  ];

  const rows = $derived(filtered.slice((page - 1) * perPage, page * perPage));

  const chips = $derived([
    { key: 'all' as ChipKey, label: $t('common.all'), count: counts.all },
    { key: 'serving' as ChipKey, label: $t('admin.customers.pppoe.v2.stat_serving'), count: counts.serving },
    { key: 'isolated' as ChipKey, label: $t('admin.customers.pppoe.v2.stat_isolated'), count: counts.isolated },
    { key: 'missing' as ChipKey, label: $t('admin.customers.pppoe.v2.stat_missing'), count: counts.missing },
    { key: 'draft' as ChipKey, label: $t('admin.customers.pppoe.v2.stat_draft'), count: counts.draft },
  ]);

  function applyChip(key: ChipKey) {
    chip = key;
    page = 1;
  }

  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  function onSearch(value: string) {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      q = value;
      page = 1;
    }, 250);
  }

  async function load() {
    try {
      /* Semua akun ditarik sekali supaya hitungan chip dan tile benar untuk
         seluruh tenant, bukan hanya 25 baris pertama seperti versi lama.
         Batas backend 1.000/permintaan (src-tauri/src/services/pagination.rs),
         jadi 1.010 akun selesai dalam 2 permintaan — sebelum batasnya dinaikkan
         ini butuh 11. */
      const [accounts, customers, routerList] = await Promise.all([
        fetchAllPages<PppoeAccountPublic>((p, per_page) =>
          api.pppoe.accounts.list({ page: p, per_page }),
        ),
        fetchAllRows<CustomerListItem>((p, per_page) =>
          api.customers.list({ page: p, perPage: per_page }),
        ).catch((e) => { console.error('customers list gagal:', e); return [] as CustomerListItem[]; }),
        api.mikrotik.routers.list().catch((e) => { console.error('routers list gagal:', e); return [] as RouterRef[]; }) as Promise<RouterRef[]>,
      ]);

      all = accounts.rows;
      /* Jujur soal kelengkapan: kalau penarikan berhenti di batas halaman,
         tile dan chip harus mengaku "minimal N", bukan mengklaim total. */
      complete = accounts.complete;
      customerName = new Map(customers.map((c) => [c.id, c.name]));
      routerName = new Map(routerList.map((r) => [r.id, r.name]));
    } catch (e) {
      err = 'Gagal memuat akun PPPoE';
      console.warn('list pppoe gagal', e);
    } finally {
      loading = false;
    }
  }

  onMount(() => void load());

  let reconciling = $state(false);
  async function reconcileAll() {
    if (!canManage || reconciling) return;
    reconciling = true;
    try {
      const ids = routers.map((r) => r.id);
      const results = await Promise.allSettled(
        ids.map((rid) => api.pppoe.accounts.reconcileRouter(rid)),
      );
      const failed = results.filter((r) => r.status === 'rejected').length;
      if (failed === 0) toast.success($t('admin.customers.pppoe.v2.t_reconciled'));
      else toast.warning($t('admin.customers.pppoe.v2.t_recon_partial', { values: {ok: ids.length - failed, failed }}));
      await load();
    } catch (e: unknown) {
      toast.error($t('admin.customers.pppoe.v2.t_reconcile_failed', { values: {message: extractApiErrorMessage(e) }}));
    } finally {
      reconciling = false;
    }
  }

  async function applyToRouter(a: PppoeAccountPublic) {
    if (!canManage || busyId) return;
    busyId = a.id;
    try {
      await api.pppoe.accounts.apply(a.id);
      toast.success($t('admin.customers.pppoe.v2.t_applied'));
      await load();
    } catch (e: unknown) {
      toast.error($t('admin.customers.pppoe.v2.t_apply_failed', { values: {message: extractApiErrorMessage(e) }}));
    } finally {
      busyId = null;
    }
  }

  async function toggleIsolate(a: PppoeAccountPublic) {
    if (!canManage || busyId) return;
    busyId = a.id;
    try {
      await api.pppoe.accounts.update(a.id, { disabled: !a.disabled });
      toast.success(a.disabled ? 'Akun diaktifkan kembali.' : 'Akun diisolir (dinonaktifkan di router).');
      await load();
    } catch (e: unknown) {
      toast.error($t('admin.customers.pppoe.v2.t_toggle_failed', { values: {message: extractApiErrorMessage(e) }}));
    } finally {
      busyId = null;
    }
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    deleting = true;
    try {
      await api.pppoe.accounts.delete(deleteTarget.id);
      toast.success($t('admin.customers.pppoe.v2.t_deleted'));
      showDeleteConfirm = false;
      deleteTarget = null;
      await load();
    } catch (e: unknown) {
      toast.error($t('admin.customers.pppoe.v2.t_delete_failed', { values: {message: extractApiErrorMessage(e) }}));
    } finally {
      deleting = false;
    }
  }
</script>

<AppShell title="PPPoE">
  <PageHeader
    title={ $t('admin.customers.pppoe.title') }
    eyebrow={loading
      ? $t('admin.customers.pppoe.v2.eyebrow_loading')
      : complete
        ? $t('admin.customers.pppoe.v2.eyebrow_count', { values: {n: counts.all }})
        : $t('admin.customers.pppoe.v2.eyebrow_partial', { values: {n: counts.all }})}
    desc={ $t('admin.customers.pppoe.v2.desc') }
  >
    {#snippet actions()}
      {#if canManage}
        <Button icon="refresh" loading={reconciling} onclick={reconcileAll}>{ $t('admin.customers.pppoe.v2.reconcile_btn') }</Button>
        <Button variant="primary" icon="plus" onclick={openCreate}>{ $t('admin.customers.pppoe.actions.add') }</Button>
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

  <Card class="mb-4">
    <div class="grid grid-cols-2 gap-5 lg:grid-cols-4">
      <StatTile
        label={ $t('admin.customers.pppoe.v2.stat_serving') }
        value={String(counts.serving)}
        hint={loading ? $t('admin.customers.pppoe.v2.counting') : $t('admin.customers.pppoe.v2.hint_from', { values: { n: counts.all } })}
        tone="positive"
      />
      <StatTile
        label={ $t('admin.customers.pppoe.v2.stat_isolated') }
        value={String(counts.isolated)}
        hint={loading ? $t('admin.customers.pppoe.v2.counting') : $t('admin.customers.pppoe.v2.hint_isolated')}
      />
      <StatTile
        label={ $t('admin.customers.pppoe.v2.stat_missing') }
        value={String(counts.missing)}
        hint={loading ? $t('admin.customers.pppoe.v2.counting') : $t('admin.customers.pppoe.v2.hint_missing')}
        tone={counts.missing > 0 ? 'negative' : 'positive'}
      />
      <StatTile
        label={ $t('admin.customers.pppoe.v2.stat_routers') }
        value={String(routers.length)}
        hint={loading
          ? $t('admin.customers.pppoe.v2.counting')
          : routers[0]
            ? $t('admin.customers.pppoe.v2.hint_top_router', { values: { name: routers[0].name, n: routers[0].n } })
            : $t('admin.customers.pppoe.v2.hint_no_router')}
      />
    </div>
  </Card>

  {#if !loading && counts.missing > 0}
    <!-- Drift nyata: akun ada di aplikasi tapi tidak ada di perangkat. -->
    <div class="mb-4">
      <AttentionPanel
        title={ $t('admin.customers.pppoe.v2.reconcile_title') }
        items={[
          {
            icon: 'router',
            title: $t('admin.customers.pppoe.v2.reconcile_items', { values: {n: counts.missing }}),
            detail:
              $t('admin.customers.pppoe.v2.reconcile_detail'),
            action: $t('admin.customers.pppoe.v2.see_list'),
            severity: 'high',
          },
        ]}
      />
    </div>
  {/if}

  <div class="mb-3 flex flex-wrap items-center gap-2">
    {#each chips as c (c.key)}
      <button
        onclick={() => applyChip(c.key)}
        aria-pressed={chip === c.key}
        class="focus-ring flex h-8 items-center gap-1.5 rounded-lg px-3 text-base
          {chip === c.key
          ? 'bg-ink-900 font-medium text-white'
          : 'bg-white text-ink-700 ring-1 ring-inset ring-ink-200 hover:bg-ink-50'}"
      >
        {c.label}
        <span class="num text-sm {chip === c.key ? 'text-ink-300' : 'text-ink-400'}">{c.count}</span>
      </button>
    {/each}

    <div class="ml-auto flex items-center gap-2">
      {#if routers.length > 1}
        <select
          bind:value={routerFilter}
          onchange={() => (page = 1)}
          aria-label={ $t('admin.network.incidents.ui.router_filter') }
          class="h-8 rounded-lg bg-white px-2.5 text-base text-ink-900 ring-1 ring-inset ring-ink-200 focus:ring-brand-600 focus:outline-none"
        >
          <option value="">{ $t('admin.network.incidents.ui.all_routers') }</option>
          {#each routers as r (r.id)}
            <option value={r.id}>{r.name} ({r.n})</option>
          {/each}
        </select>
      {/if}

      <div class="relative w-full sm:w-56">
        <span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-400">
          <Icon name="search" size={15} />
        </span>
        <input
          oninput={(e) => onSearch((e.currentTarget as HTMLInputElement).value)}
          type="search"
          placeholder={ $t('admin.customers.pppoe.v2.search_ph') }
          aria-label={ $t('admin.customers.pppoe.search') }
          class="h-8 w-full rounded-lg bg-white pr-3 pl-8 text-base text-ink-900 ring-1 ring-inset ring-ink-200 placeholder:text-ink-400 focus:ring-brand-600 focus:outline-none"
        />
      </div>
    </div>
  </div>

  <Card padded={false}>
    <DataTable
      {columns}
      {rows}
      {loading}
      page={page}
      total={filtered.length}
      pageSize={perPage}
      onpage={(p) => (page = p)}
      onpagesize={(n) => {
        perPage = n;
        page = 1;
      }}
      emptyTitle={ $t('admin.customers.pppoe.v2.empty_match') }
      emptyHint={q ? $t('common.no_results_for', { values: { q } }) : $t('network.pppoe_v2.empty_hint')}
    >
      {#snippet cell(a, column)}
        {#if column.key === 'username'}
          <span class="num font-medium text-ink-900">{a.username}</span>
        {:else if column.key === 'customer'}
          <span class="block max-w-xs truncate">{customerName.get(a.customer_id) ?? '—'}</span>
        {:else if column.key === 'status'}
          <Badge tone={healthMeta[healthOf(a)].tone} label={healthMeta[healthOf(a)].label} />
        {:else if column.key === 'router'}
          {routerName.get(a.router_id) ?? '—'}
        {:else if column.key === 'ip'}
          <span class="num text-sm">{a.remote_address || a.address_pool || '—'}</span>
        {:else if column.key === 'actions'}
          <RowActions
            primary={{
              label: $t('common.edit'),
              icon: 'cog',
              onclick: () => void openEdit(a),
            }}
            rest={canManage
              ? [
                  {
                    label: $t('network.pppoe_v2.apply_router'),
                    icon: 'refresh',
                    disabled: busyId === a.id,
                    onclick: () => void applyToRouter(a),
                  },
                  {
                    label: a.disabled ? $t('admin.customers.pppoe.v2.act_enable') : $t('admin.customers.pppoe.v2.act_isolate'),
                    icon: a.disabled ? 'check' : 'alert',
                    disabled: busyId === a.id,
                    onclick: () => void toggleIsolate(a),
                  },
                  {
                    label: $t('admin.customers.pppoe.v2.act_delete'),
                    icon: 'close',
                    danger: true,
                    onclick: () => {
                      deleteTarget = a;
                      showDeleteConfirm = true;
                    },
                  },
                ]
              : []}
          />
        {/if}
      {/snippet}
    </DataTable>
  </Card>

<ConfirmDialog
  bind:show={showDeleteConfirm}
  type="danger"
  title={ $t('admin.customers.pppoe.v2.confirm_delete_title') }
  message={deleteTarget ? $t('admin.customers.pppoe.v2.delete_msg', { values: { u: deleteTarget.username } }) : ''}
  confirmText="Hapus"
  loading={deleting}
  onconfirm={() => void confirmDelete()}
/>
{#if PppoeAccountModalComponent}
  <PppoeAccountModalComponent
    mode="create"
    bind:show={showCreate}
    {saving}
    {routerOptions}
    {customerOptions}
    {locationOptions}
    {packageOptions}
    {packageSelectionHasMissingMapping}
    bind:formRouterId
    bind:formCustomerId
    bind:formLocationId
    bind:formUsername
    bind:formPassword
    bind:formPackageId
    bind:formComment
    bind:formDisabled
    bind:formAccountSource
    onRouterChange={() => {
      formPackageId = '';
      formRouterProfileName = '';
      formRemoteAddress = '';
      formAddressPool = '';
      void loadRouterPackages(formRouterId);
    }}
    onCustomerChange={() => {
      formLocationId = '';
      void loadLocations(formCustomerId);
    }}
    onPackageChange={() => applyPackageToForm(formPackageId)}
    onSubmit={submitCreate}
    {sourceLabel}
    {sourceDisabledHintLabel}
    {sourceCreateActionLabel}
  />
  <PppoeAccountModalComponent
    mode="edit"
    bind:show={showEdit}
    {saving}
    {routerOptions}
    {customerOptions}
    {locationOptions}
    {packageOptions}
    {packageSelectionHasMissingMapping}
    bind:formRouterId
    bind:formCustomerId
    bind:formLocationId
    bind:formUsername
    bind:formPassword
    bind:formPackageId
    bind:formComment
    bind:formDisabled
    bind:formAccountSource
    routerDisplayName={formRouterId ? routerName.get(formRouterId) || '' : ''}
    customerDisplayName={formCustomerId ? customerName.get(formCustomerId) || '' : ''}
    locationDisplayName={formLocationId ? locations.find((l) => l.id === formLocationId)?.label || '' : ''}
    onRouterChange={() => {
      formPackageId = '';
      void loadRouterPackages(formRouterId);
    }}
    onCustomerChange={() => {
      formLocationId = '';
      void loadLocations(formCustomerId);
    }}
    onPackageChange={() => applyPackageToForm(formPackageId)}
    onSubmit={submitEdit}
    {sourceLabel}
    {sourceDisabledHintLabel}
    {sourceCreateActionLabel}
  />
{/if}
</AppShell>
