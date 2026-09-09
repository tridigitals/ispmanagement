<script lang="ts">
  /*
    DHCP Static v2.

    Versi lama: `(app)/admin/network/dhcp-static/+page.svelte` (1.538 baris).

    Temuan yang dikunci gelombang ini:

    1. HAPUS TIDAK MEMBERSIHKAN ROUTER. DELETE lama hanya menghapus baris
       DB — lease statis dan simple queue TETAP ADA di MikroTik selamanya,
       dan reconcile tidak lagi tahu entri itu pernah ada (baris acuannya
       hilang). Sekarang delete memanggil lease/remove + queue/remove
       sebelum baris DB hilang; kegagalan router dicatat di pesan audit.
    2. ERROR QUEUE DITELAN DIAM-DIAM. apply_service menyimpan
       queue_last_error tapi tetap balas 200 — toast "berhasil" padahal
       pembatas bandwidth gagal dibuat. Kini error queue ikut dilaporkan.
    3. Search `ILIKE '%' || $q || '%'` tanpa escape wildcard (bug ketiga
       setelah audit-logs & services): cari "%" mencocokkan seluruh tabel.
    4. Hapus id tak dikenal = sukses hampa + audit log palsu -> 404.
    5. FE lama punya dua dimensi sync (lease & queue) tapi hanya pill
       Present/Missing; lease_last_error / queue_last_error tidak pernah
       ditampilkan. v2 merangkumnya jadi satu badge + modal detail.
  */
  import { onMount } from 'svelte';
  import { can } from '$lib/stores/auth';
  import { api, type DhcpStaticServicePublic, type IspPackage } from '$lib/api/client';
  import type { CustomerSubscriptionView } from '$lib/api/client';
  import type { MikrotikDhcpServerOption } from '$lib/api/mikrotik';
  import AppShell from '$lib/components/ds/AppShell.svelte';
  import PageHeader from '$lib/components/ds/PageHeader.svelte';
  import StatTile from '$lib/components/ds/StatTile.svelte';
  import DataTable from '$lib/components/ds/DataTable.svelte';
  import type { Column } from '$lib/components/ds/table-types';
  import Badge from '$lib/components/ds/Badge.svelte';
  import Button from '$lib/components/ds/Button.svelte';
  import Field from '$lib/components/ds/Field.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import RowActions from '$lib/components/ds/RowActions.svelte';
  import type { RowAction } from '$lib/components/ds/RowActions.svelte';
  import AttentionPanel from '$lib/components/ds/AttentionPanel.svelte';
  import type { AttentionItem } from '$lib/components/ds/AttentionPanel.svelte';
  import {
    dhcpSyncLabel,
    dhcpSyncState,
    dhcpSyncSummary,
    dhcpSyncTone,
    friendlyDhcpError,
    type DhcpSyncState,
  } from '$lib/utils/dhcpInsights';
  import {
    formatDhcpStaticMacAddressInput,
    normalizeDhcpStaticMacAddress,
    validateDhcpStaticIpv4Address,
    validateDhcpStaticQueueRateLimit,
  } from '$lib/utils/dhcpStaticValidation';
  import { buildDhcpStaticQueueRateLimitPresets } from '$lib/utils/dhcpStaticQueuePresets';
  import { extractApiErrorMessage } from '$lib/api/core';
  import { toast } from '$lib/stores/toast';

  import { t } from 'svelte-i18n';
  type RouterRow = { id: string; name: string };
  type CustomerRow = { id: string; name: string };
  type LocationRow = { id: string; label: string };

  let rows = $state<DhcpStaticServicePublic[]>([]);
  let total = $state(0);
  let page = $state(1);
  const perPage = 25;
  let loading = $state(true);
  let loadError = $state<string | null>(null);

  let search = $state('');
  let filterRouterId = $state('');
  let syncFilter = $state<'all' | DhcpSyncState | 'problem'>('all');

  let routers = $state<RouterRow[]>([]);
  let customers = $state<CustomerRow[]>([]);
  let packages = $state<IspPackage[]>([]);
  const customerName = $derived(new Map(customers.map((c) => [c.id, c.name])));
  const packageName = $derived(new Map(packages.map((p) => [p.id, p.name])));
  const routerName = $derived(new Map(routers.map((r) => [r.id, r.name])));

  const canManage = $derived($can('manage', 'dhcp_static'));

  let attention = $state<AttentionItem[]>([]);

  const stats = $derived.by(() => {
    const s = { all: total, synced: 0, partial: 0, problem: 0, disabled: 0 };
    for (const r of rows) {
      const st = dhcpSyncState(r);
      if (st === 'missing' || st === 'error') s.problem += 1;
      else s[st] += 1;
    }
    return s;
  });

  const visibleRows = $derived(
    syncFilter === 'all'
      ? rows
      : syncFilter === 'problem'
        ? rows.filter((r) => ['missing', 'error'].includes(dhcpSyncState(r)))
        : rows.filter((r) => dhcpSyncState(r) === syncFilter),
  );

  let timer: ReturnType<typeof setTimeout> | undefined;

  async function load() {
    loading = true;
    loadError = null;
    try {
      const res = await api.dhcpStatic.services.list({
        q: search.trim() || undefined,
        router_id: filterRouterId || undefined,
        page,
        per_page: perPage,
      });
      rows = res.data || [];
      total = res.total || 0;
      attention = [];
      const broken = rows.filter((r) => ['missing', 'error'].includes(dhcpSyncState(r)));
      if (broken.length > 0) {
        attention.push({
          severity: 'high',
          icon: 'alert',
          title: $t('admin.network.dhcp_static.v2.attn_broken_title', { values: { n: broken.length } }),
          detail: $t('admin.network.dhcp_static.v2.attn_broken_detail'),
          action: $t('admin.network.dhcp_static.v2.attn_broken_action'),
          href: '/v2/admin/network/dhcp-static?sync=problem',
        });
      }
    } catch (e) {
      loadError = friendlyDhcpError(extractApiErrorMessage(e));
    } finally {
      loading = false;
    }
  }

  function onSearch() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      page = 1;
      void load();
    }, 350);
  }

  onMount(() => {
    void (async () => {
      if (!$can('read', 'dhcp_static') && !$can('manage', 'dhcp_static')) {
        loadError = 'Anda tidak punya akses ke halaman ini.';
        loading = false;
        return;
      }
      const q = new URLSearchParams(location.search).get('sync');
      if (q === 'problem' || q === 'partial' || q === 'synced' || q === 'disabled') {
        syncFilter = q;
      }
      const [routersRes, customersRes, packagesRes] = await Promise.all([
        api.mikrotik.routers.list().catch((e) => { console.error('routers list gagal:', e); return [] as RouterRow[]; }),
        api.customers.list({ page: 1, perPage: 1000 }).catch((e) => { console.error('customers list gagal:', e); return { data: [] as CustomerRow[] }; }),
        api.ispPackages.packages.list({ page: 1, per_page: 500 }).catch((e) => { console.error('packages list gagal:', e); return { data: [] as IspPackage[] }; }),
      ]);
      routers = (routersRes || []).map((r) => ({ id: r.id, name: r.name }));
      customers = (customersRes?.data || []).map((c) => ({ id: c.id, name: c.name }));
      packages = packagesRes?.data || [];
      await load();
    })();
  });

  // ── modal form ────────────────────────────────────────────────────────
  let formOpen = $state(false);
  let editRow = $state<DhcpStaticServicePublic | null>(null);
  let saving = $state(false);
  let formError = $state<string | null>(null);
  let fCustomerId = $state('');
  let fSubscriptionId = $state('');
  let fLocationId = $state('');
  let fPackageId = $state('');
  let fRouterId = $state('');
  let fServer = $state('');
  let fMac = $state('');
  let fIp = $state('');
  let fComment = $state('');
  let fDisabled = $state(false);
  let fQueueMode = $state<'none' | 'simple_queue'>('none');
  let fQueueRate = $state('');
  let errMac = $state<string | null>(null);
  let errIp = $state<string | null>(null);
  let errQueue = $state<string | null>(null);
  let errRequired = $state<string | null>(null);

  let locations = $state<LocationRow[]>([]);
  let subs = $state<CustomerSubscriptionView[]>([]);
  let dhcpServers = $state<MikrotikDhcpServerOption[]>([]);
  let loadingServers = $state(false);
  let serverToken = 0;

  const dhcpStaticPackages = $derived(
    packages.filter((p) => p.service_type === 'internet_pppoe' && p.provisioning_type === 'dhcp_static'),
  );
  const queuePresets = $derived(
    buildDhcpStaticQueueRateLimitPresets(packages.find((p) => p.id === fPackageId) ?? {}),
  );

  function resetForm() {
    editRow = null;
    fCustomerId = fSubscriptionId = fLocationId = fPackageId = fRouterId = fServer = '';
    fMac = fIp = fComment = fQueueRate = '';
    fDisabled = false;
    fQueueMode = 'none';
    errMac = errIp = errQueue = errRequired = null;
    formError = null;
    locations = [];
    subs = [];
    dhcpServers = [];
  }

  function openCreate() {
    resetForm();
    formOpen = true;
  }

  async function openEdit(row: DhcpStaticServicePublic) {
    resetForm();
    editRow = row;
    fCustomerId = row.customer_id;
    fSubscriptionId = row.subscription_id;
    fLocationId = row.location_id;
    fPackageId = row.package_id;
    fRouterId = row.router_id;
    fServer = row.dhcp_server_name;
    fMac = row.mac_address;
    fIp = row.ip_address;
    fComment = row.comment || '';
    fDisabled = Boolean(row.disabled);
    fQueueMode = row.queue_mode === 'simple_queue' ? 'simple_queue' : 'none';
    fQueueRate = row.queue_rate_limit || '';
    await loadCustomerScope(row.customer_id);
    await loadServers(row.router_id, true);
    formOpen = true;
  }

  async function loadCustomerScope(customerId: string) {
    if (!customerId) {
      locations = [];
      subs = [];
      return;
    }
    const [locRes, subRes] = await Promise.all([
      api.customers.locations.list(customerId).catch((e) => { console.error('locations list gagal:', e); return [] as LocationRow[]; }),
      api.customers.subscriptions.list(customerId, { page: 1, per_page: 200 }).catch((e) => { console.error('subscriptions list gagal:', e); return { data: [] as CustomerSubscriptionView[] }; }),
    ]);
    locations = (locRes || []).map((l) => ({ id: l.id, label: l.label }));
    subs = subRes?.data || [];
  }

  async function loadServers(routerId: string, preserve: boolean) {
    const token = ++serverToken;
    if (!routerId) {
      dhcpServers = [];
      if (!preserve) fServer = '';
      return;
    }
    loadingServers = true;
    try {
      const servers = await api.mikrotik.routers.dhcpServers(routerId);
      if (token !== serverToken) return;
      dhcpServers = (servers || []).filter((s) => !s.disabled);
      if (!dhcpServers.some((s) => s.name === fServer)) {
        if (dhcpServers.length === 1) fServer = dhcpServers[0].name;
        else if (!preserve) fServer = '';
      }
    } catch {
      if (token !== serverToken) return;
      dhcpServers = [];
      if (!preserve) fServer = '';
    } finally {
      if (token === serverToken) loadingServers = false;
    }
  }

  function onCustomerChange() {
    fSubscriptionId = '';
    fLocationId = '';
    fPackageId = '';
    void loadCustomerScope(fCustomerId);
  }

  function onSubscriptionChange() {
    const sub = subs.find((s) => s.id === fSubscriptionId);
    if (!sub) return;
    fLocationId = sub.location_id;
    fPackageId = sub.package_id;
    if (sub.router_id) {
      void loadServers(sub.router_id, true).then(() => {
        fRouterId = sub.router_id || '';
      });
    }
  }

  function validateForm(): boolean {
    errMac = errIp = errQueue = errRequired = null;
    if (!fCustomerId || !fSubscriptionId || !fRouterId || !fServer || !fPackageId) {
      errRequired = 'Pelanggan, langganan, router, DHCP server, dan paket wajib diisi.';
      return false;
    }
    const mac = normalizeDhcpStaticMacAddress(fMac);
    if (mac.error || !mac.value) {
      errMac = 'Format MAC tidak valid. Contoh: AA:BB:CC:DD:EE:FF';
      return false;
    }
    fMac = mac.value;
    if (validateDhcpStaticIpv4Address(fIp)) {
      errIp = 'IP IPv4 tidak valid. Contoh: 10.10.20.55';
      return false;
    }
    if (fQueueMode === 'simple_queue' && validateDhcpStaticQueueRateLimit(fQueueRate)) {
      errQueue = 'Rate limit wajib untuk simple_queue. Contoh: 10M/10M';
      return false;
    }
    return true;
  }

  async function submitForm() {
    if (!validateForm() || saving) return;
    saving = true;
    formError = null;
    try {
      if (editRow) {
        await api.dhcpStatic.services.update(editRow.id, {
          router_id: fRouterId,
          package_id: fPackageId,
          dhcp_server_name: fServer,
          mac_address: fMac,
          ip_address: fIp,
          comment: fComment || null,
          disabled: fDisabled,
          queue_mode: fQueueMode,
          queue_rate_limit: fQueueMode === 'simple_queue' ? fQueueRate : null,
        });
        toast.success($t('admin.network.dhcp_static.v2.t_updated'));
      } else {
        await api.dhcpStatic.services.create({
          subscription_id: fSubscriptionId,
          router_id: fRouterId,
          customer_id: fCustomerId,
          location_id: fLocationId,
          package_id: fPackageId,
          dhcp_server_name: fServer,
          mac_address: fMac,
          ip_address: fIp,
          comment: fComment || null,
          disabled: fDisabled,
          queue_mode: fQueueMode,
          queue_rate_limit: fQueueMode === 'simple_queue' ? fQueueRate : null,
        });
        toast.success($t('admin.network.dhcp_static.v2.t_created'));
      }
      formOpen = false;
      await load();
    } catch (e) {
      formError = friendlyDhcpError(extractApiErrorMessage(e));
    } finally {
      saving = false;
    }
  }

  // ── apply / reconcile / delete ────────────────────────────────────────
  let busyId = $state<string | null>(null);

  async function applyRow(row: DhcpStaticServicePublic) {
    busyId = row.id;
    try {
      await api.dhcpStatic.services.apply(row.id);
      toast.success($t('admin.network.dhcp_static.v2.t_applied', { values: { mac: row.mac_address } }));
    } catch (e) {
      toast.error(friendlyDhcpError(extractApiErrorMessage(e)));
    } finally {
      busyId = null;
      await load();
    }
  }

  async function toggleDisabled(row: DhcpStaticServicePublic) {
    busyId = row.id;
    try {
      await api.dhcpStatic.services.update(row.id, { disabled: !row.disabled });
      toast.success(row.disabled ? $t('admin.network.dhcp_static.v2.t_enabled') : $t('admin.network.dhcp_static.v2.t_disabled'));
      await load();
    } catch (e) {
      toast.error(friendlyDhcpError(extractApiErrorMessage(e)));
    } finally {
      busyId = null;
    }
  }

  let reconcileBusy = $state(false);
  async function reconcile() {
    if (!filterRouterId) {
      toast.error($t('admin.network.dhcp_static.v2.t_recon_pick'));
      return;
    }
    reconcileBusy = true;
    try {
      const res: any = await api.dhcpStatic.services.reconcileRouter(filterRouterId);
      toast.success($t('admin.network.dhcp_static.v2.t_recon_done', { values: { n: res?.updated ?? 0 } }));
      await load();
    } catch (e) {
      toast.error(friendlyDhcpError(extractApiErrorMessage(e)));
    } finally {
      reconcileBusy = false;
    }
  }

  let deleteTarget = $state<DhcpStaticServicePublic | null>(null);
  let deleteOpen = $state(false);
  let deleting = $state(false);
  let deleteError = $state<string | null>(null);

  function confirmDelete(row: DhcpStaticServicePublic) {
    deleteError = null;
    deleteTarget = row;
    deleteOpen = true;
  }

  async function doDelete() {
    if (!deleteTarget || deleting) return;
    deleting = true;
    deleteError = null;
    try {
      await api.dhcpStatic.services.delete(deleteTarget.id);
      toast.success($t('admin.network.dhcp_static.v2.t_deleted'));
      deleteTarget = null;
      await load();
    } catch (e) {
      deleteError = friendlyDhcpError(extractApiErrorMessage(e));
    } finally {
      deleting = false;
    }
  }

  // ── modal detail sync ─────────────────────────────────────────────────
  let detailRow = $state<DhcpStaticServicePublic | null>(null);
  let detailOpen = $state(false);

  function openDetail(row: DhcpStaticServicePublic) {
    detailRow = row;
    detailOpen = true;
  }

  // ── tabel ─────────────────────────────────────────────────────────────
  const columns = $derived<Column[]>([
    { key: 'customer_id', label: $t('common.customer'), width: '180px' },
    { key: 'mac_address', label: $t('admin.network.dhcp_static.v2.col_mac'), width: '150px' },
    { key: 'ip_address', label: $t('admin.network.dhcp_static.v2.col_ip'), width: '130px' },
    { key: 'router_id', label: $t('admin.network.dhcp_static.v2.col_router'), width: '190px' },
    { key: 'package_id', label: $t('common.package'), width: '150px' },
    { key: 'sync', label: $t('admin.network.dhcp_static.v2.col_sync'), width: '150px' },
    { key: 'actions', label: '', width: '96px', align: 'right' },
  ]);

  function rowRest(row: DhcpStaticServicePublic): RowAction[] {
    const acts: RowAction[] = [];
    if (canManage) {
      acts.push({
        label: $t('admin.network.dhcp_static.v2.act_apply'),
        icon: 'zap',
        disabled: busyId === row.id,
        onclick: () => void applyRow(row),
      });
      acts.push({
        label: row.disabled ? $t('admin.network.dhcp_static.v2.act_enable') : $t('admin.network.dhcp_static.v2.act_disable'),
        icon: 'check',
        disabled: busyId === row.id,
        onclick: () => void toggleDisabled(row),
      });
      acts.push({ label: $t('admin.network.dhcp_static.v2.act_edit'), icon: 'cog', onclick: () => void openEdit(row) });
      acts.push({ label: $t('common.delete'), icon: 'close', danger: true, onclick: () => confirmDelete(row) });
    }
    return acts;
  }

  function fmtDate(iso: string | null): string {
    if (!iso) return '—';
    try {
      return new Date(iso).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' });
    } catch {
      return iso;
    }
  }
</script>

<AppShell title={ $t('admin.network.dhcp_static.title') }>
  <PageHeader title={ $t('admin.network.dhcp_static.title') } desc={ $t('admin.network.dhcp_static.v2.desc') }>
    {#snippet actions()}
      {#if canManage}
        <Button variant="primary" icon="plus" onclick={openCreate}>{ $t('admin.network.dhcp_static.v2.add_service') }</Button>
      {/if}
    {/snippet}
  </PageHeader>

  {#if attention.length}
    <AttentionPanel items={attention} title={ $t('admin.network.dhcp_static.v2.attention') } />
  {/if}

  <div class="mt-4 grid grid-cols-2 gap-3 md:grid-cols-5">
    {#each [
      { st: 'all', label: $t('admin.network.dhcp_static.stats.total'), hint: $t('admin.network.dhcp_static.v2.tile_hint') },
      { st: 'synced', label: $t('admin.network.dhcp_static.v2.tile_synced'), hint: $t('admin.network.dhcp_static.v2.tile_hint_synced') },
      { st: 'partial', label: $t('admin.network.dhcp_static.v2.tile_partial'), hint: $t('admin.network.dhcp_static.v2.tile_hint_partial') },
      { st: 'problem', label: $t('admin.network.dhcp_static.v2.tile_problem'), hint: $t('admin.network.dhcp_static.v2.tile_hint_problem') },
      { st: 'disabled', label: $t('admin.network.dhcp_static.stats.disabled'), hint: $t('admin.network.dhcp_static.v2.tile_hint_disabled') },
    ] as tile}
      <button
        type="button"
        class="rounded-xl text-left focus-ring {syncFilter === tile.st ? 'ring-2 ring-ink-900' : ''}"
        onclick={() => (syncFilter = tile.st as typeof syncFilter)}
        aria-pressed={syncFilter === tile.st}
      >
        <StatTile
          label={tile.label}
          value={String(stats[tile.st as keyof typeof stats] ?? 0)}
          hint={tile.hint}
          tone={tile.st === 'partial' ? 'warning' : tile.st === 'problem' ? 'negative' : 'neutral'}
        />
      </button>
    {/each}
  </div>

  <div class="mt-4 flex flex-wrap items-center gap-2">
    <div class="relative min-w-[220px] flex-1">
      <input
        type="search"
        class="h-9 w-full rounded-lg border border-ink-200 bg-white pl-9 pr-3 text-sm text-ink-900 placeholder:text-ink-400"
        placeholder={ $t('admin.network.dhcp_static.v2.search_ph') }
        bind:value={search}
        oninput={onSearch}
        aria-label={ $t('admin.network.dhcp_static.v2.search_aria') }
      />
      <svg class="pointer-events-none absolute left-2.5 top-2.5 h-4 w-4 text-ink-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM21 21l-4.35-4.35" /></svg>
    </div>
    <select
      class="h-9 rounded-lg border border-ink-200 bg-white px-2 text-sm text-ink-900"
      bind:value={filterRouterId}
      onchange={() => { page = 1; void load(); }}
      aria-label={ $t('admin.network.incidents.ui.router_filter') }
    >
      <option value="">{ $t('admin.network.dhcp_static.filters.all_routers') }</option>
      {#each routers as r (r.id)}
        <option value={r.id}>{r.name}</option>
      {/each}
    </select>
    {#if canManage}
      <Button variant="ghost" icon="refresh" disabled={reconcileBusy || !filterRouterId} onclick={() => void reconcile()}>
        Rekonsiliasi router
      </Button>
    {/if}
  </div>

  {#if loadError}
    <div class="mt-4 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700">
      {loadError}
    </div>
  {/if}

  <div class="mt-4">
    <DataTable {columns} rows={visibleRows} {loading} pageSize={perPage} page={page} {total} onpage={(p) => { page = p; void load(); }} footNote={`${total} ${$t('admin.network.dhcp_static.v2.foot_prefix')}${search || filterRouterId ? $t('admin.network.dhcp_static.v2.foot_filtered') : ''}.`}>
      {#snippet cell(row: DhcpStaticServicePublic, col: Column)}
        {#if col.key === 'customer_id'}
          <span class="text-sm">{customerName.get(row.customer_id) || row.customer_id.slice(0, 8)}</span>
        {:else if col.key === 'mac_address'}
          <code class="rounded bg-ink-50 px-1.5 py-0.5 text-xs">{row.mac_address}</code>
        {:else if col.key === 'ip_address'}
          <code class="rounded bg-ink-50 px-1.5 py-0.5 text-xs">{row.ip_address}</code>
        {:else if col.key === 'router_id'}
          <span class="text-sm">{routerName.get(row.router_id) || '—'}<span class="text-ink-400"> · {row.dhcp_server_name}</span></span>
        {:else if col.key === 'package_id'}
          <span class="text-sm">{packageName.get(row.package_id) || '—'}</span>
        {:else if col.key === 'sync'}
          {@const st = dhcpSyncState(row)}
          <button type="button" class="focus-ring inline-flex min-h-6 items-center rounded-md" onclick={() => openDetail(row)} aria-label={ $t('admin.network.dhcp_static.v2.sync_aria') }>
            <Badge tone={dhcpSyncTone(st)} label={dhcpSyncLabel(st)} />
          </button>
        {:else if col.key === 'actions'}
          <RowActions primary={{ label: $t('admin.network.dhcp_static.v2.act_view_sync'), icon: 'clock', onclick: () => openDetail(row) }} rest={rowRest(row)} />
        {/if}
      {/snippet}
    </DataTable>
  </div>
</AppShell>

<!-- modal buat/sunting -->
<Modal title={editRow ? $t('admin.network.dhcp_static.v2.edit_title') : $t('admin.network.dhcp_static.v2.create_title')} bind:show={formOpen}>
  <div class="space-y-3">
    {#if errRequired}
      <p class="rounded-lg bg-red-50 p-2 text-sm text-red-700">{errRequired}</p>
    {/if}
    <div class="grid grid-cols-1 gap-3 md:grid-cols-2">
      <Field
        stacked
        id="d-cust"
        label={ $t('common.customer') }
        value={fCustomerId}
        type="select"
        options={[{ value: '', label: '— pilih pelanggan —' }, ...customers.map((c) => ({ value: c.id, label: c.name }))]}
        disabled={Boolean(editRow)}
        onchange={(v) => { fCustomerId = v; onCustomerChange(); }}
      />
      <Field
        stacked
        id="d-sub"
        label={ $t('admin.customers.tabs.subscriptions') }
        value={fSubscriptionId}
        type="select"
        options={[{ value: '', label: '— pilih langganan —' }, ...subs.map((x) => ({ value: x.id, label: x.package_name || x.id.slice(0, 8) }))]}
        disabled={!fCustomerId || Boolean(editRow)}
        onchange={(v) => { fSubscriptionId = v; onSubscriptionChange(); }}
      />
      <Field
        stacked
        id="d-router"
        label={ $t('admin.network.dhcp_static.filters.router') }
        value={fRouterId}
        type="select"
        options={[{ value: '', label: '— pilih router —' }, ...routers.map((r) => ({ value: r.id, label: r.name }))]}
        onchange={(v) => { fRouterId = v; void loadServers(fRouterId, false); }}
      />
      <Field
        stacked
        id="d-server"
        label={ $t('admin.network.dhcp_static.columns.server') }
        value={fServer}
        type="select"
        options={[{ value: '', label: loadingServers ? $t('admin.network.dhcp_static.v2.server_ph') : $t('admin.network.dhcp_static.v2.server_dash_ph') }, ...dhcpServers.map((x) => ({ value: x.name, label: x.interface ? `${x.name} • ${x.interface}` : x.name }))]}
        disabled={!fRouterId || loadingServers}
        onchange={(v) => (fServer = v)}
      />
      <Field
        stacked
        id="d-mac"
        label={ $t('admin.network.dhcp_static.v2.field_mac') }
        value={fMac}
        placeholder="AA:BB:CC:DD:EE:FF"
        error={errMac}
        onchange={(v) => (fMac = formatDhcpStaticMacAddressInput(v))}
      />
      <Field stacked id="d-ip" label={ $t('admin.network.dhcp_static.v2.field_ip') } value={fIp} placeholder="10.10.20.55" error={errIp} onchange={(v) => (fIp = v)} />
      <Field
        stacked
        id="d-pkg"
        label={ $t('common.package') }
        value={fPackageId}
        type="select"
        options={[{ value: '', label: $t('admin.network.dhcp_static.v2.pkg_dash_ph') }, ...dhcpStaticPackages.map((x) => ({ value: x.id, label: x.name }))]}
        onchange={(v) => (fPackageId = v)}
      />
      <Field
        stacked
        id="d-qmode"
        label={ $t('admin.network.dhcp_static.v2.field_queue') }
        value={fQueueMode}
        type="select"
        options={[{ value: 'none', label: $t('admin.network.dhcp_static.v2.q_none') }, { value: 'simple_queue', label: $t('admin.network.dhcp_static.fields.simple_queue') }]}
        onchange={(v) => (fQueueMode = v as 'none' | 'simple_queue')}
      />
    </div>
    {#if fQueueMode === 'simple_queue'}
      <Field
        stacked
        id="d-queue"
        label={ $t('admin.network.dhcp_static.v2.rate_field') }
        value={fQueueRate}
        placeholder="10M/10M"
        help={$t('admin.network.dhcp_static.v2.queue_help', { values: { presets: queuePresets.join(', ') } })}
        error={errQueue}
        onchange={(v) => (fQueueRate = v)}
      />
    {/if}
    <Field stacked id="d-comment" label={ $t('common.notes') } value={fComment} placeholder={ $t('admin.network.dhcp_static.v2.comment_ph') } onchange={(v) => (fComment = v)} />
    <Field
      stacked
      id="d-disabled"
      label={ $t('admin.network.dhcp_static.v2.field_disable') }
      type="toggle"
      value={fDisabled ? 'true' : 'false'}
      help={ $t('admin.network.dhcp_static.v2.disable_help') }
      onchange={(v) => (fDisabled = v === 'true')}
    />
    {#if formError}
      <p class="rounded-lg bg-red-50 p-2 text-sm text-red-700">{formError}</p>
    {/if}
    <div class="flex justify-end gap-2 pt-1">
      <Button variant="ghost" onclick={() => (formOpen = false)}>{ $t('common.cancel') }</Button>
      <Button variant="primary" disabled={saving} onclick={() => void submitForm()}>
        {saving ? $t('admin.network.dhcp_static.v2.saving') : editRow ? $t('common.save') : $t('common.create')}
      </Button>
    </div>
  </div>
</Modal>

<!-- modal detail sinkron -->
<Modal title={ $t('admin.network.dhcp_static.v2.sync_title') } bind:show={detailOpen}>
  {#if detailRow}
    {@const st = dhcpSyncState(detailRow)}
    <div class="space-y-3 text-sm">
      <div class="flex items-center gap-2">
        <Badge tone={dhcpSyncTone(st)} label={dhcpSyncLabel(st)} />
        <code>{detailRow.mac_address} → {detailRow.ip_address}</code>
      </div>
      <p>{dhcpSyncSummary(detailRow)}</p>
      <dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-ink-500">
        <dt>{ $t('admin.network.dhcp_static.v2.dt_router') }</dt><dd class="text-ink-900">{routerName.get(detailRow.router_id) || '—'} · {detailRow.dhcp_server_name}</dd>
        <dt>{ $t('admin.network.dhcp_static.v2.dt_lease_ref') }</dt><dd class="text-ink-900">{detailRow.lease_router_ref || '—'}</dd>
        <dt>{ $t('admin.network.dhcp_static.v2.dt_sync_last') }</dt><dd class="text-ink-900">{fmtDate(detailRow.lease_last_sync_at)}</dd>
        <dt>Queue</dt><dd class="text-ink-900">{detailRow.queue_mode === 'none' ? $t('admin.network.dhcp_static.v2.queue_none') : `${detailRow.queue_name || '—'} (${detailRow.queue_rate_limit || '—'})`}</dd>
      </dl>
      {#if detailRow.lease_last_error}
        <p class="rounded-lg bg-red-50 p-2 text-red-700">{ $t('admin.network.dhcp_static.v2.err_lease') } {detailRow.lease_last_error}</p>
      {/if}
      {#if detailRow.queue_last_error}
        <p class="rounded-lg bg-red-50 p-2 text-red-700">{ $t('admin.network.dhcp_static.v2.err_queue') } {detailRow.queue_last_error}</p>
      {/if}
      {#if canManage}
        <div class="flex justify-end gap-2 pt-1">
          <Button variant="ghost" onclick={() => (detailOpen = false)}>{ $t('common.close') }</Button>
          <Button variant="primary" icon="zap" disabled={busyId === detailRow.id} onclick={() => { const r = detailRow; detailOpen = false; detailRow = null; if (r) void applyRow(r); }}>
            { $t('admin.network.dhcp_static.v2.reapply') }
          </Button>
        </div>
      {:else}
        <div class="flex justify-end pt-1"><Button variant="ghost" onclick={() => (detailOpen = false)}>{ $t('common.close') }</Button></div>
      {/if}
    </div>
  {/if}
</Modal>

<!-- modal hapus -->
<Modal title={ $t('admin.network.dhcp_static.v2.delete_title') } bind:show={deleteOpen}>
  {#if deleteTarget}
    <div class="space-y-3 text-sm">
      <p>
        {$t('admin.network.dhcp_static.v2.delete_body_a')}<code>{deleteTarget.mac_address} → {deleteTarget.ip_address}</code>{$t('admin.network.dhcp_static.v2.delete_body_b')}<strong>{routerName.get(deleteTarget.router_id) || deleteTarget.router_id}</strong>.
      </p>
      {#if deleteError}
        <p class="rounded-lg bg-red-50 p-2 text-red-700">{deleteError}</p>
      {/if}
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => (deleteOpen = false)}>{ $t('common.cancel') }</Button>
        <Button variant="danger" disabled={deleting} onclick={() => void doDelete()}>
          {deleting ? $t('admin.network.dhcp_static.v2.deleting') : $t('common.delete')}
        </Button>
      </div>
    </div>
  {/if}
</Modal>

<style>
  .dh-input {
    height: 36px;
    width: 100%;
    border-radius: 8px;
    border: 1px solid var(--ds-border, #e4e4e7);
    background: #fff;
    padding: 0 10px;
    font-size: 13px;
    color: #18181b;
  }
  .dh-input:focus {
    outline: 2px solid #8b9cff;
    outline-offset: 1px;
  }
</style>
