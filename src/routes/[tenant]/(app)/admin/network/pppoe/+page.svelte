<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { t } from 'svelte-i18n';
  import { can, user, tenant } from '$lib/stores/auth';
  import { api, type IspPackageRouterMappingView, type PppoeAccountPublic } from '$lib/api/client';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import Select2 from '$lib/components/ui/Select2.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import NetworkFilterPanel from '$lib/components/network/NetworkFilterPanel.svelte';
  import NetworkPageHeader from '$lib/components/network/NetworkPageHeader.svelte';
  import { timeAgo } from '$lib/utils/date';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';

  type RouterRow = { id: string; name: string; host?: string; port?: number };
  type CustomerRow = { id: string; name: string };
  type LocationRow = { id: string; label: string };
  type ProfileSuggestion = { id: string; name: string };
  type PoolSuggestion = { id: string; name: string };
  const IMPORT_PLACEHOLDER_CUSTOMER_NAME = 'Imported (Unassigned)';

  let loading = $state(true);
  let error = $state('');
  let accounts = $state<PppoeAccountPublic[]>([]);
  let routers = $state<RouterRow[]>([]);
  let customers = $state<CustomerRow[]>([]);
  let locations = $state<LocationRow[]>([]);
  let refreshing = $state(false);
  let autoApplyOnSave = $state(false);
  let retryingIds = $state<string[]>([]);

  let q = $state('');
  let routerId = $state('');
  let status = $state<'any' | 'present' | 'missing'>('any');
  let disabled = $state<'any' | 'enabled' | 'disabled'>('any');
  let provisioning = $state<'any' | 'applied' | 'draft' | 'failed'>('any');

  // Create/Edit modal state
  let showCreate = $state(false);
  let showEdit = $state(false);
  let saving = $state(false);
  let editRow = $state<PppoeAccountPublic | null>(null);

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

  let profileSuggestions = $state<ProfileSuggestion[]>([]);
  let poolSuggestions = $state<PoolSuggestion[]>([]);
  let loadingRouterMeta = $state(false);
  const routerMetaCache = new Map<string, { profiles: ProfileSuggestion[]; pools: PoolSuggestion[] }>();

  const profileOptions = $derived.by(() => {
    const base = (profileSuggestions || []).map((p) => ({ label: p.name, value: p.name }));
    const cur = formRouterProfileName?.trim();
    if (cur && !base.some((o) => o.value === cur)) return [{ label: cur, value: cur }, ...base];
    return base;
  });

  const poolOptions = $derived.by(() => {
    const base = (poolSuggestions || []).map((p) => ({ label: p.name, value: p.name }));
    const cur = formAddressPool?.trim();
    if (cur && !base.some((o) => o.value === cur)) return [{ label: cur, value: cur }, ...base];
    return base;
  });

  let routerPackageMappings = $state<IspPackageRouterMappingView[]>([]);
  const packageOptions = $derived.by(() => {
    // Show only active mapped packages for selected router
    const seen = new Set<string>();
    const out: Array<{ label: string; value: string }> = [];
    for (const m of routerPackageMappings) {
      if (!m?.package_id || seen.has(m.package_id)) continue;
      seen.add(m.package_id);
      out.push({ label: m.package_name, value: m.package_id });
    }
    return out;
  });

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);

  const routerName = (id: string) => routers.find((r) => r.id === id)?.name || '-';
  const customerName = (id: string) => customers.find((c) => c.id === id)?.name || '-';
  const isImportPlaceholderCustomer = (id: string) => customerName(id) === IMPORT_PLACEHOLDER_CUSTOMER_NAME;
  const customerLabel = (id: string) =>
    isImportPlaceholderCustomer(id)
      ? ($t('admin.network.pppoe.import.fields.unassigned') || 'Unassigned')
      : customerName(id);
  const routerHost = (id: string) => routers.find((r) => r.id === id)?.host || '';
  const routerPort = (id: string) => routers.find((r) => r.id === id)?.port || 0;

  const routerOptions = $derived.by(() => routers.map((r) => ({ label: r.name, value: r.id })));
  const customerOptions = $derived.by(() =>
    customers
      .filter((c) => c.name !== IMPORT_PLACEHOLDER_CUSTOMER_NAME)
      .map((c) => ({ label: c.name, value: c.id })),
  );
  const locationOptions = $derived.by(() => locations.map((l) => ({ label: l.label, value: l.id })));

  const viewRows = $derived.by(() =>
    accounts.filter((a) => {
      if (status === 'present' && !a.router_present) return false;
      if (status === 'missing' && a.router_present) return false;
      if (disabled === 'enabled' && a.disabled) return false;
      if (disabled === 'disabled' && !a.disabled) return false;
      if (provisioning !== 'any') {
        const st = provisioningState(a);
        if (st !== provisioning) return false;
      }
      return true;
    }),
  );

  function clearFilters() {
    q = '';
    routerId = '';
    status = 'any';
    disabled = 'any';
    provisioning = 'any';
    void loadAccounts();
  }

  const stats = $derived.by(() => {
    const total = viewRows.length;
    const present = viewRows.filter((a) => a.router_present).length;
    const missing = total - present;
    const disabledCount = viewRows.filter((a) => a.disabled).length;
    return { total, present, missing, disabled: disabledCount };
  });

  const columns = $derived.by(() => [
    { key: 'username', label: $t('admin.network.pppoe.columns.username') || 'Username' },
    { key: 'customer', label: $t('admin.network.pppoe.columns.customer') || 'Customer' },
    { key: 'router', label: $t('admin.network.pppoe.columns.router') || 'Router' },
    { key: 'provisioning', label: $t('admin.network.pppoe.columns.provisioning') || 'Provisioning' },
    { key: 'sync', label: $t('admin.network.pppoe.columns.sync') || 'Sync' },
    { key: 'actions', label: '', align: 'right' as const, width: '300px' },
  ]);

  onMount(() => {
    if (!$can('read', 'pppoe') && !$can('manage', 'pppoe')) {
      goto('/unauthorized');
      return;
    }
    void load();
    if ($can('read', 'settings') || $can('update', 'settings')) {
      void loadProvisioningSetting();
    } else {
      autoApplyOnSave = false;
    }
  });

  async function load() {
    loading = true;
    error = '';
    try {
      await Promise.all([loadRouters(), loadCustomers(), loadAccounts()]);
    } catch (e: any) {
      error = String(e?.message || e || '');
    } finally {
      loading = false;
    }
  }

  async function loadProvisioningSetting() {
    if (!$can('read', 'settings') && !$can('update', 'settings')) {
      autoApplyOnSave = false;
      return;
    }
    try {
      const raw = await api.settings.getValue('pppoe_auto_apply_on_save_enabled');
      const val = String(raw || '')
        .trim()
        .toLowerCase();
      autoApplyOnSave = val === 'true' || val === '1' || val === 'yes' || val === 'on';
    } catch {
      autoApplyOnSave = false;
    }
  }

  async function loadRouters() {
    try {
      routers = (await api.mikrotik.routers.list()) as any;
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function loadCustomers() {
    try {
      // We only need id+name for mapping. For now, load first 1000.
      const res = await api.customers.list({ page: 1, perPage: 1000 });
      customers = (res.data || []).map((c) => ({ id: c.id, name: c.name }));
    } catch (e: any) {
      // Non-critical; list can still show ids.
    }
  }

  async function loadLocations(customerId: string) {
    if (!customerId) {
      locations = [];
      return;
    }
    try {
      const rows = await api.customers.locations.list(customerId);
      locations = (rows || []).map((l: any) => ({ id: l.id, label: l.label }));
    } catch {
      locations = [];
    }
  }

  async function loadRouterMeta(routerId: string) {
    if (!routerId) {
      profileSuggestions = [];
      poolSuggestions = [];
      return;
    }
    const cached = routerMetaCache.get(routerId);
    if (cached) {
      profileSuggestions = cached.profiles;
      poolSuggestions = cached.pools;
      return;
    }
    loadingRouterMeta = true;
    try {
      const [profiles, pools] = await Promise.all([
        api.mikrotik.routers.pppProfiles(routerId),
        api.mikrotik.routers.ipPools(routerId),
      ]);
      const mappedProfiles = (profiles || []).map((p: any) => ({ id: p.id, name: p.name }));
      const mappedPools = (pools || []).map((p: any) => ({ id: p.id, name: p.name }));
      routerMetaCache.set(routerId, { profiles: mappedProfiles, pools: mappedPools });
      profileSuggestions = mappedProfiles;
      poolSuggestions = mappedPools;
    } catch {
      profileSuggestions = [];
      poolSuggestions = [];
    } finally {
      loadingRouterMeta = false;
    }
  }

  async function loadRouterPackages(routerId: string) {
    if (!routerId) {
      routerPackageMappings = [];
      return;
    }
    try {
      routerPackageMappings = await api.ispPackages.routerMappings.list({ router_id: routerId });
    } catch {
      routerPackageMappings = [];
    }
  }

  function maybeAutoSelectPackageFromProfile() {
    const profile = formRouterProfileName?.trim();
    if (!formRouterId || !profile) return;
    if (formPackageId) return;

    const matches = routerPackageMappings.filter((m) => (m.router_profile_name || '') === profile);
    if (matches.length === 1) {
      formPackageId = matches[0].package_id;
      applyPackageToForm(formPackageId);
      return;
    }

    if (!formAddressPool) {
      const withPool = matches.find((m) => m.address_pool);
      if (withPool?.address_pool) formAddressPool = withPool.address_pool;
    }
  }

  function applyPackageToForm(pkgId: string) {
    if (!pkgId) return;
    const m = routerPackageMappings.find((x) => x.package_id === pkgId);
    if (!m) return;
    // Prefill but allow overrides.
    formRouterProfileName = m.router_profile_name || '';
    if (m.address_pool) {
      formAddressPool = m.address_pool;
      formRemoteAddress = '';
    }
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
    locations = [];
    profileSuggestions = [];
    poolSuggestions = [];
    routerPackageMappings = [];
    editRow = null;
  }

  async function openCreate() {
    if (!$can('manage', 'pppoe')) {
      toast.error($t('common.forbidden') || 'Forbidden');
      return;
    }
    resetForm();
    showCreate = true;
  }

  async function openEdit(row: PppoeAccountPublic) {
    if (!$can('manage', 'pppoe')) {
      toast.error($t('common.forbidden') || 'Forbidden');
      return;
    }

    resetForm();
    editRow = row;
    formRouterId = row.router_id;
    formCustomerId = row.customer_id;
    formLocationId = row.location_id;
    formUsername = row.username;
    formPassword = '';
    formPackageId = row.package_id || '';
    formRouterProfileName = row.router_profile_name || '';
    formRemoteAddress = row.remote_address || '';
    formAddressPool = row.address_pool || '';
    formDisabled = Boolean(row.disabled);
    formComment = row.comment || '';
    showEdit = true;

    await Promise.all([loadLocations(row.customer_id), loadRouterMeta(row.router_id), loadRouterPackages(row.router_id)]);
  }

  async function submitCreate() {
    if (saving) return;
    if (!formRouterId || !formCustomerId || !formLocationId || !formUsername.trim() || !formPassword) return;

    saving = true;
    try {
      const created = await api.pppoe.accounts.create({
        router_id: formRouterId,
        customer_id: formCustomerId,
        location_id: formLocationId,
        username: formUsername.trim(),
        password: formPassword,
        package_id: formPackageId || null,
        router_profile_name: formRouterProfileName.trim() || null,
        remote_address: formRemoteAddress.trim() || null,
        address_pool: formAddressPool.trim() || null,
        disabled: formDisabled,
        comment: formComment.trim() || null,
      });
      if (autoApplyOnSave && created?.id) {
        try {
          await api.pppoe.accounts.apply(created.id);
          toast.success(
            $t('admin.network.pppoe.toasts.auto_applied') || 'Saved and automatically applied to router',
          );
        } catch (e: any) {
          toast.error(
            $t('admin.network.pppoe.toasts.auto_apply_failed', {
              values: { message: e?.message || e },
            }) || `Saved, but auto-apply failed: ${e?.message || e}`,
          );
        }
      }
      toast.success($t('admin.customers.pppoe.toasts.created') || 'PPPoE account created');
      showCreate = false;
      await loadAccounts();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      saving = false;
    }
  }

  async function submitEdit() {
    if (saving) return;
    if (!editRow) return;
    if (!formUsername.trim()) return;

    saving = true;
    try {
      const updated = await api.pppoe.accounts.update(editRow.id, {
        username: formUsername.trim(),
        password: formPassword || undefined,
        package_id: formPackageId || null,
        router_profile_name: formRouterProfileName.trim() || null,
        remote_address: formRemoteAddress.trim() || null,
        address_pool: formAddressPool.trim() || null,
        disabled: formDisabled,
        comment: formComment.trim() || null,
      });
      if (autoApplyOnSave && updated?.id) {
        try {
          await api.pppoe.accounts.apply(updated.id);
          toast.success(
            $t('admin.network.pppoe.toasts.auto_applied') || 'Saved and automatically applied to router',
          );
        } catch (e: any) {
          toast.error(
            $t('admin.network.pppoe.toasts.auto_apply_failed', {
              values: { message: e?.message || e },
            }) || `Saved, but auto-apply failed: ${e?.message || e}`,
          );
        }
      }
      toast.success($t('admin.customers.pppoe.toasts.updated') || 'PPPoE account updated');
      showEdit = false;
      await loadAccounts();
    } catch (e: any) {
      toast.error(e?.message || e);
    } finally {
      saving = false;
    }
  }

  async function deleteAccount(row: PppoeAccountPublic) {
    if (!$can('manage', 'pppoe')) return;
    if (!confirm($t('admin.customers.pppoe.confirm_delete') || 'Delete this PPPoE account?')) return;
    try {
      await api.pppoe.accounts.delete(row.id);
      toast.success($t('common.deleted') || 'Deleted');
      await loadAccounts();
    } catch (e: any) {
      toast.error(e?.message || e);
    }
  }

  async function loadAccounts() {
    refreshing = true;
    try {
      // Fetch in chunks so large imports (hundreds/thousands) show fully even if the backend enforces paging.
      const requestedPerPage = 500;
      const first = await api.pppoe.accounts.list({
        q: q.trim() || undefined,
        router_id: routerId || undefined,
        page: 1,
        per_page: requestedPerPage,
      });

      let all = first.data || [];
      const total = Number(first.total || all.length);
      // Use the server's effective page size (it may clamp/ignore our requested per_page).
      const effectivePerPage =
        Number((first as any).per_page || all.length || requestedPerPage) || requestedPerPage;

      // If more pages exist, fetch the rest sequentially (keeps logic simple and avoids hammering the server).
      if (all.length < total) {
        const maxPages = Math.ceil(total / effectivePerPage);
        for (let p = 2; p <= maxPages; p++) {
          const next = await api.pppoe.accounts.list({
            q: q.trim() || undefined,
            router_id: routerId || undefined,
            page: p,
            per_page: effectivePerPage,
          });
          const chunk = next.data || [];
          if (chunk.length === 0) break;
          all = [...all, ...chunk];
          if (all.length >= total) break;
        }
      }

      accounts = all;
    } catch (e: any) {
      const msg = String(e?.message || e || 'Failed to load PPPoE accounts');
      error = msg;
      toast.error(msg);
    } finally {
      refreshing = false;
    }
  }

  async function reconcileAll() {
    const routerIds = Array.from(new Set(viewRows.map((a) => a.router_id).filter(Boolean)));
    if (routerIds.length === 0) return;
    try {
      for (const rid of routerIds) {
        await api.pppoe.accounts.reconcileRouter(rid);
      }
      toast.success($t('admin.network.pppoe.toasts.reconciled') || 'Reconciled router state');
      await loadAccounts();
    } catch (e: any) {
      toast.error(
        $t('admin.network.pppoe.toasts.reconcile_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    }
  }

  async function apply(row: PppoeAccountPublic) {
    try {
      await api.pppoe.accounts.apply(row.id);
      toast.success($t('admin.network.pppoe.toasts.applied') || 'Applied to router');
      await loadAccounts();
    } catch (e: any) {
      toast.error(
        $t('admin.network.pppoe.toasts.apply_failed', { values: { message: e?.message || e } }) ||
          `Failed: ${e?.message || e}`,
      );
    }
  }

  function provisioningState(row: PppoeAccountPublic): 'retrying' | 'failed' | 'applied' | 'draft' {
    if (retryingIds.includes(row.id)) return 'retrying';
    if (row.last_error && row.last_error.trim()) return 'failed';
    if (row.router_present && row.router_secret_id) return 'applied';
    return 'draft';
  }

  function provisioningLabel(state: 'retrying' | 'failed' | 'applied' | 'draft') {
    if (state === 'retrying') return $t('admin.network.pppoe.provisioning.retrying') || 'Retrying';
    if (state === 'failed') return $t('admin.network.pppoe.provisioning.failed') || 'Failed';
    if (state === 'applied') return $t('admin.network.pppoe.provisioning.applied') || 'Applied';
    return $t('admin.network.pppoe.provisioning.draft') || 'Draft';
  }

  const retryCandidates = $derived.by(() =>
    viewRows.filter((r) => {
      const st = provisioningState(r);
      return st === 'failed' || st === 'draft';
    }),
  );

  async function retryApplyBatch() {
    if (!$can('manage', 'pppoe')) return;
    if (retryCandidates.length === 0) return;

    let ok = 0;
    let fail = 0;

    for (const row of retryCandidates) {
      retryingIds = Array.from(new Set([...retryingIds, row.id]));
      try {
        await api.pppoe.accounts.apply(row.id);
        ok += 1;
      } catch {
        fail += 1;
      } finally {
        retryingIds = retryingIds.filter((id) => id !== row.id);
      }
    }

    if (ok > 0) {
      toast.success(
        $t('admin.network.pppoe.toasts.retry_batch_ok', { values: { count: ok } }) ||
          `${ok} account(s) applied`,
      );
    }
    if (fail > 0) {
      toast.error(
        $t('admin.network.pppoe.toasts.retry_batch_fail', { values: { count: fail } }) ||
          `${fail} account(s) failed to apply`,
      );
    }
    await loadAccounts();
  }
</script>

<div class="page-content fade-in">
  <NetworkPageHeader
    title={$t('admin.network.pppoe.title') || 'PPPoE'}
    subtitle={$t('admin.network.pppoe.subtitle') || 'Tenant-wide view of PPPoE accounts across routers.'}
  >
    {#snippet actions()}
      {#if $can('manage', 'pppoe') && autoApplyOnSave}
        <span class="chip active">{$t('admin.network.pppoe.auto_apply_on') || 'Auto-apply ON'}</span>
      {/if}
      <button class="btn ghost" type="button" onclick={loadAccounts} disabled={refreshing}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      {#if $can('manage', 'pppoe')}
        <button class="btn" type="button" onclick={openCreate}>
          <Icon name="plus" size={16} />
          {$t('admin.customers.pppoe.actions.add') || 'Add PPPoE'}
        </button>
      {/if}
      {#if $can('manage', 'pppoe')}
        <button
          class="btn ghost"
          type="button"
          onclick={() => goto(`${tenantPrefix}/admin/network/pppoe/import`)}
          title={$t('admin.network.pppoe.import.title') || 'Import PPPoE from MikroTik'}
        >
          <Icon name="download" size={16} />
          {$t('admin.network.pppoe.import.cta') || 'Import'}
        </button>
      {/if}
      {#if $can('manage', 'pppoe')}
        <button
          class="btn ghost"
          type="button"
          onclick={reconcileAll}
          disabled={refreshing || viewRows.length === 0}
          title={$t('admin.network.pppoe.actions.reconcile') || 'Reconcile'}
        >
          <Icon name="refresh-cw" size={16} />
          {$t('admin.network.pppoe.actions.reconcile') || 'Reconcile'}
        </button>
      {/if}
      {#if $can('manage', 'pppoe')}
        <button
          class="btn ghost"
          type="button"
          onclick={retryApplyBatch}
          disabled={refreshing || retryCandidates.length === 0}
          title={$t('admin.network.pppoe.actions.retry_apply_batch') || 'Retry apply'}
        >
          <Icon name="rotate-cw" size={16} />
          {$t('admin.network.pppoe.actions.retry_apply_batch') || 'Retry apply'}
          <span class="pill mono">{retryCandidates.length}</span>
        </button>
      {/if}
      {#if q.trim() || routerId || status !== 'any' || disabled !== 'any' || provisioning !== 'any'}
        <button class="btn ghost" type="button" onclick={clearFilters}>
          <Icon name="eraser" size={16} />
          {$t('common.clear') || 'Clear'}
        </button>
      {/if}
    {/snippet}
  </NetworkPageHeader>

  <div class="stats">
    <div class="stat-card">
      <div class="stat-top">
        <span>{$t('admin.network.pppoe.stats.total') || 'Total'}</span>
        <Icon name="key" size={16} />
      </div>
      <div class="stat-value">{stats.total}</div>
    </div>
    <div class="stat-card tone-ok">
      <div class="stat-top">
        <span>{$t('admin.network.pppoe.stats.present') || 'On router'}</span>
        <Icon name="check-circle" size={16} />
      </div>
      <div class="stat-value">{stats.present}</div>
    </div>
    <div class="stat-card tone-warn">
      <div class="stat-top">
        <span>{$t('admin.network.pppoe.stats.missing') || 'Missing'}</span>
        <Icon name="alert-triangle" size={16} />
      </div>
      <div class="stat-value">{stats.missing}</div>
    </div>
    <div class="stat-card tone-muted">
      <div class="stat-top">
        <span>{$t('admin.network.pppoe.stats.disabled') || 'Disabled'}</span>
        <Icon name="pause" size={16} />
      </div>
      <div class="stat-value">{stats.disabled}</div>
    </div>
  </div>

  <div class="filters-wrap">
    <NetworkFilterPanel>
      <div class="control control-wide">
        <label for="pppoe-search">{$t('common.search') || 'Search'}</label>
        <label class="search-wrap" for="pppoe-search">
          <Icon name="search" size={14} />
          <input
            id="pppoe-search"
            type="text"
            placeholder={$t('admin.network.pppoe.search') || 'Search PPPoE...'}
            value={q}
            oninput={(e) => {
              q = (e.currentTarget as HTMLInputElement).value;
              void loadAccounts();
            }}
          />
          {#if q.trim()}
            <button
              class="clear"
              type="button"
              onclick={() => {
                q = '';
                void loadAccounts();
              }}
              aria-label={$t('common.clear') || 'Clear'}
            >
              <Icon name="x" size={14} />
            </button>
          {/if}
        </label>
      </div>

      <div class="control">
        <label for="pppoe-filter-sync">{$t('admin.network.pppoe.filters.sync') || 'Sync'}</label>
        <select id="pppoe-filter-sync" class="input" bind:value={status}>
          <option value="any">{$t('admin.network.pppoe.filters.any') || 'Any'}</option>
          <option value="present">{$t('admin.network.pppoe.filters.present') || 'On router'}</option>
          <option value="missing">{$t('admin.network.pppoe.filters.missing') || 'Missing'}</option>
        </select>
      </div>

      <div class="control">
        <label for="pppoe-filter-state">{$t('admin.network.pppoe.filters.disabled') || 'State'}</label>
        <select id="pppoe-filter-state" class="input" bind:value={disabled}>
          <option value="any">{$t('admin.network.pppoe.filters.any') || 'Any'}</option>
          <option value="enabled">{$t('admin.network.pppoe.filters.enabled') || 'Enabled'}</option>
          <option value="disabled">{$t('admin.network.pppoe.filters.disabled_only') || 'Disabled'}</option>
        </select>
      </div>

      <div class="control">
        <label for="pppoe-filter-prov">{$t('admin.network.pppoe.filters.provisioning') || 'Provisioning'}</label>
        <select id="pppoe-filter-prov" class="input" bind:value={provisioning}>
          <option value="any">{$t('admin.network.pppoe.filters.any') || 'Any'}</option>
          <option value="applied">{$t('admin.network.pppoe.provisioning.applied') || 'Applied'}</option>
          <option value="draft">{$t('admin.network.pppoe.provisioning.draft') || 'Draft'}</option>
          <option value="failed">{$t('admin.network.pppoe.provisioning.failed') || 'Failed'}</option>
        </select>
      </div>

      <div class="control control-wide">
        <label for="pppoe-filter-router">{$t('admin.network.pppoe.filters.router') || 'Router'}</label>
        <Select2
          bind:value={routerId}
          options={routerOptions}
          placeholder={($t('admin.network.pppoe.filters.all') || 'All') + '...'}
          width="100%"
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => void loadAccounts()}
        />
      </div>
    </NetworkFilterPanel>
  </div>

  <div class="table-wrap">
    <div class="table-top">
      <span class="muted">{viewRows.length} {$t('common.results') || 'results'}</span>
    </div>

    {#if error}
      <div class="error-banner">
        <Icon name="alert-triangle" size={18} />
        <span>{error}</span>
      </div>
    {/if}

    <Table
      columns={columns}
      data={viewRows}
      loading={loading}
      emptyText={$t('admin.network.pppoe.empty') || 'No PPPoE accounts.'}
      pagination
    >
      {#snippet cell({ item, key })}
        {@const row = item as PppoeAccountPublic}
        {#if key === 'username'}
          <div class="stack">
            <div class="name">{row.username}</div>
            <div class="meta">
              {#if row.disabled}
                <span class="badge warn">{$t('common.disabled') || 'Disabled'}</span>
              {:else}
                <span class="badge ok">{$t('common.active') || 'Active'}</span>
              {/if}
              <span class="pill">
                {$t('admin.customers.pppoe.fields.profile') || 'Profile'}:
                {row.router_profile_name || '-'}
              </span>
              <span class="pill">
                {$t('admin.customers.pppoe.fields.remote_address') || 'Remote'}:
                {row.remote_address || row.address_pool || '-'}
              </span>
            </div>
          </div>
        {:else if key === 'customer'}
          {@const isPlaceholderCustomer = isImportPlaceholderCustomer(row.customer_id)}
          <div class="stack">
            <div class="name">{customerLabel(row.customer_id)}</div>
            {#if !isPlaceholderCustomer}
              <div class="meta">
                <span class="pill mono" title={row.customer_id}>{row.customer_id.slice(0, 8)}…</span>
                <span class="pill mono" title={row.location_id}>
                  {$t('sidebar.locations') || 'Locations'}: {row.location_id.slice(0, 8)}…
                </span>
              </div>
            {/if}
          </div>
        {:else if key === 'router'}
          <div class="stack">
            <div class="name">{routerName(row.router_id)}</div>
            <div class="meta">
              {#if routerHost(row.router_id)}
                <span class="pill mono">{routerHost(row.router_id)}:{routerPort(row.router_id) || ''}</span>
              {/if}
              <span class="pill mono" title={row.router_id}>{row.router_id.slice(0, 8)}…</span>
            </div>
          </div>
        {:else if key === 'sync'}
          <div class="stack">
            <div class="meta">
              {#if row.router_present}
                <span class="badge ok">{$t('admin.network.pppoe.sync.present') || 'On router'}</span>
              {:else}
                <span class="badge warn">{$t('admin.network.pppoe.sync.missing') || 'Missing'}</span>
              {/if}
              <span class="pill mono">{row.last_sync_at ? timeAgo(row.last_sync_at) : '-'}</span>
            </div>
            {#if row.last_error}
              <div class="error-line" title={row.last_error}>
                <Icon name="alert-triangle" size={14} />
                <span class="error-text">{row.last_error}</span>
              </div>
            {/if}
          </div>
        {:else if key === 'provisioning'}
          {@const state = provisioningState(row)}
          <div class="stack">
            <span
              class="badge"
              class:ok={state === 'applied'}
              class:warn={state === 'draft' || state === 'retrying'}
              class:danger={state === 'failed'}
            >
              {provisioningLabel(state)}
            </span>
            {#if state === 'failed' && row.last_error}
              <span class="error-text" title={row.last_error}>{row.last_error}</span>
            {/if}
          </div>
        {:else if key === 'actions'}
          {@const isPlaceholderCustomer = isImportPlaceholderCustomer(row.customer_id)}
          <div class="row-actions">
            {#if !isPlaceholderCustomer}
              <button
                class="btn-icon"
                title={$t('admin.network.pppoe.actions.open_customer') || 'Open customer'}
                onclick={() => row.customer_id && goto(`${tenantPrefix}/admin/customers/${row.customer_id}`)}
              >
                <Icon name="external-link" size={16} />
              </button>
              <button
                class="btn-icon"
                title={$t('admin.network.pppoe.actions.open_customer_billing') || 'Open customer billing'}
                onclick={() => row.customer_id && goto(`${tenantPrefix}/admin/customers/${row.customer_id}?tab=billing`)}
              >
                <Icon name="file-text" size={16} />
              </button>
            {/if}
            {#if $can('manage', 'pppoe')}
              <button
                class="btn-icon"
                title={$t('admin.network.pppoe.actions.apply') || 'Apply to router'}
                onclick={() => apply(row)}
              >
                <Icon name="send" size={16} />
              </button>
              <button class="btn-icon" title={$t('common.edit') || 'Edit'} onclick={() => openEdit(row)}>
                <Icon name="edit" size={16} />
              </button>
              <button class="btn-icon danger" title={$t('common.delete') || 'Delete'} onclick={() => deleteAccount(row)}>
                <Icon name="trash-2" size={16} />
              </button>
            {/if}
          </div>
        {:else}
          {item[key] ?? ''}
        {/if}
      {/snippet}
    </Table>
  </div>
</div>

<Modal
  show={showCreate}
  title={$t('admin.customers.pppoe.new.title') || 'Add PPPoE account'}
  width="760px"
  onclose={() => (showCreate = false)}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.router') || 'Router'}</span>
        <Select2
          bind:value={formRouterId}
          options={routerOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => {
            // Reset router-scoped selections when router changes
            formPackageId = '';
            formRouterProfileName = '';
            formRemoteAddress = '';
            formAddressPool = '';
            void loadRouterMeta(formRouterId);
            void loadRouterPackages(formRouterId);
          }}
        />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.customer') || 'Customer'}</span>
        <Select2
          bind:value={formCustomerId}
          options={customerOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          maxItems={5000}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => {
            formLocationId = '';
            void loadLocations(formCustomerId);
          }}
        />
      </label>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.package') || 'Package'}</span>
      <Select2
        bind:value={formPackageId}
        options={packageOptions}
        placeholder={($t('common.select') || 'Select') + '...'}
        width="100%"
        disabled={!formRouterId || packageOptions.length === 0}
        searchPlaceholder={$t('common.search') || 'Search'}
        noResultsText={$t('common.no_results') || 'No results'}
        onchange={() => applyPackageToForm(formPackageId)}
      />
      <div class="field-hint">
        {$t('admin.network.pppoe.form.package_hint') ||
          'If you select a package, profile/pool will be prefilled for the selected router (you can still override).'}
      </div>
    </label>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.location') || 'Location'}</span>
        <Select2
          bind:value={formLocationId}
          options={locationOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!formCustomerId}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
        />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.username') || 'Username'}</span>
        <input class="input" bind:value={formUsername} />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.password') || 'Password'}</span>
        <input class="input" type="password" bind:value={formPassword} />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.profile') || 'Profile'}</span>
        <Select2
          bind:value={formRouterProfileName}
          options={profileOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!formRouterId || profileOptions.length === 0}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => maybeAutoSelectPackageFromProfile()}
        />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.remote_address') || 'Remote IP'}</span>
        <input class="input mono" bind:value={formRemoteAddress} placeholder="10.10.10.10" disabled={!formRouterId} />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.pool') || 'Address pool'}</span>
        <Select2
          bind:value={formAddressPool}
          options={poolOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!formRouterId || poolOptions.length === 0}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
        />
      </label>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.comment') || 'Comment'}</span>
      <input class="input" bind:value={formComment} />
    </label>

    <div class="toggle-row">
      <div class="toggle-text">
        <div class="toggle-title">{$t('admin.customers.pppoe.fields.disabled') || 'Disabled'}</div>
        <div class="toggle-sub">
          {$t('admin.network.pppoe.form.disabled_hint') || 'Disable this PPPoE account (will be applied to router when you click Apply).'}
        </div>
      </div>
      <Toggle bind:checked={formDisabled} ariaLabel={$t('admin.customers.pppoe.fields.disabled') || 'Disabled'} />
    </div>

    <div class="actions">
      <button class="btn ghost" onclick={() => (showCreate = false)} disabled={saving}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button
        class="btn"
        onclick={submitCreate}
        disabled={saving || !formRouterId || !formCustomerId || !formLocationId || !formUsername.trim() || !formPassword}
      >
        <Icon name="plus" size={16} />
        {$t('common.create') || 'Create'}
      </button>
    </div>

    {#if loadingRouterMeta}
      <div class="hint">
        <span class="spin"><Icon name="refresh-cw" size={14} /></span>
        <span>{$t('common.loading') || 'Loading...'} suggestions…</span>
      </div>
    {/if}
  </div>
</Modal>

<Modal
  show={showEdit}
  title={$t('admin.customers.pppoe.edit.title') || 'Edit PPPoE account'}
  width="760px"
  onclose={() => (showEdit = false)}
>
  <div class="form">
    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.router') || 'Router'}</span>
        <Select2
          bind:value={formRouterId}
          options={routerOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => {
            formPackageId = '';
            formRouterProfileName = '';
            formRemoteAddress = '';
            formAddressPool = '';
            void loadRouterMeta(formRouterId);
            void loadRouterPackages(formRouterId);
          }}
        />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.customer') || 'Customer'}</span>
        <input class="input" value={customerLabel(formCustomerId)} disabled />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.location') || 'Location'}</span>
        <input
          class="input"
          value={locations.find((l) => l.id === formLocationId)?.label ||
            (formLocationId ? formLocationId.slice(0, 8) + '…' : '—')}
          disabled
        />
      </label>
      <div></div>
    </div>

    <label>
      <span>{$t('admin.customers.pppoe.fields.package') || 'Package'}</span>
      <Select2
        bind:value={formPackageId}
        options={packageOptions}
        placeholder={($t('common.select') || 'Select') + '...'}
        width="100%"
        disabled={packageOptions.length === 0}
        searchPlaceholder={$t('common.search') || 'Search'}
        noResultsText={$t('common.no_results') || 'No results'}
        onchange={() => applyPackageToForm(formPackageId)}
      />
      <div class="field-hint">
        {$t('admin.network.pppoe.form.package_hint') ||
          'If you select a package, profile/pool will be prefilled for the selected router (you can still override).'}
      </div>
    </label>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.username') || 'Username'}</span>
        <input class="input" bind:value={formUsername} />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.password') || 'Password'}</span>
        <input
          class="input"
          type="password"
          bind:value={formPassword}
          placeholder={$t('admin.customers.pppoe.edit.password_hint') || 'Leave blank to keep'}
        />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.profile') || 'Profile'}</span>
        <Select2
          bind:value={formRouterProfileName}
          options={profileOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!formRouterId || profileOptions.length === 0}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
          onchange={() => maybeAutoSelectPackageFromProfile()}
        />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.remote_address') || 'Remote IP'}</span>
        <input class="input mono" bind:value={formRemoteAddress} placeholder="10.10.10.10" />
      </label>
    </div>

    <div class="grid2">
      <label>
        <span>{$t('admin.customers.pppoe.fields.pool') || 'Address pool'}</span>
        <Select2
          bind:value={formAddressPool}
          options={poolOptions}
          placeholder={($t('common.select') || 'Select') + '...'}
          width="100%"
          disabled={!formRouterId || poolOptions.length === 0}
          searchPlaceholder={$t('common.search') || 'Search'}
          noResultsText={$t('common.no_results') || 'No results'}
        />
      </label>
      <label>
        <span>{$t('admin.customers.pppoe.fields.comment') || 'Comment'}</span>
        <input class="input" bind:value={formComment} />
      </label>
    </div>

    <div class="toggle-row">
      <div class="toggle-text">
        <div class="toggle-title">{$t('admin.customers.pppoe.fields.disabled') || 'Disabled'}</div>
        <div class="toggle-sub">
          {$t('admin.network.pppoe.form.disabled_hint') || 'Disable this PPPoE account (will be applied to router when you click Apply).'}
        </div>
      </div>
      <Toggle bind:checked={formDisabled} ariaLabel={$t('admin.customers.pppoe.fields.disabled') || 'Disabled'} />
    </div>

    <div class="actions">
      <button class="btn ghost" onclick={() => (showEdit = false)} disabled={saving}>
        {$t('common.cancel') || 'Cancel'}
      </button>
      <button class="btn" onclick={submitEdit} disabled={saving || !formUsername.trim()}>
        <Icon name="check-circle" size={16} />
        {$t('common.save') || 'Save'}
      </button>
    </div>
  </div>
</Modal>

<style>
  .page-content {
    padding: 28px;
    max-width: 1460px;
    margin: 0 auto;
  }

  .filters-wrap {
    margin-bottom: 12px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--color-primary);
    color: white;
    font-weight: 800;
    cursor: pointer;
    text-decoration: none;
  }

  .btn.ghost {
    background: transparent;
    color: var(--text-primary);
  }

  .btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .search-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface), transparent 10%);
  }

  .search-wrap input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-size: 0.95rem;
    min-width: 0;
  }

  .clear {
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 10px;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
  }

  .clear:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .table-wrap {
    border: 1px solid var(--border-color);
    border-radius: 18px;
    background: var(--bg-card);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.2);
    overflow: hidden;
  }

  .table-top {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-card) 82%, transparent);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 12px;
    margin-bottom: 14px;
  }

  .stat-card {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 16px;
    padding: 14px 14px 12px;
  }

  .stat-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-secondary);
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 0.72rem;
  }

  .stat-value {
    margin-top: 10px;
    font-size: 1.6rem;
    font-weight: 950;
    color: var(--text-primary);
  }

  .tone-ok {
    border-color: color-mix(in srgb, #1fbf75 55%, var(--border-color));
  }

  .tone-warn {
    border-color: color-mix(in srgb, #ffcc66 55%, var(--border-color));
  }

  .tone-muted {
    border-color: color-mix(in srgb, var(--text-secondary) 20%, var(--border-color));
  }

  .chip {
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary);
    padding: 0.5rem 0.75rem;
    border-radius: 999px;
    font-weight: 850;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    user-select: none;
  }

  :global([data-theme='light']) .chip {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(0, 0, 0, 0.03);
  }

  .chip:hover {
    border-color: rgba(99, 102, 241, 0.35);
    color: var(--text-primary);
  }

  .chip.active {
    border-color: rgba(99, 102, 241, 0.55);
    background: rgba(99, 102, 241, 0.18);
    color: var(--text-primary);
  }

  .stack {
    display: grid;
    gap: 0.35rem;
  }

  .name {
    font-weight: 850;
    letter-spacing: 0.01em;
  }

  .meta {
    opacity: 0.9;
    font-size: 12px;
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }
  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn-icon {
    border: 1px solid var(--border-color);
    background: color-mix(in srgb, var(--bg-surface) 70%, transparent);
    color: var(--text-primary);
    border-radius: 12px;
    width: 36px;
    height: 36px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .btn-icon:hover {
    background: var(--bg-hover);
  }
  .badge {
    font-size: 12px;
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    font-weight: 700;
  }
  .badge.ok {
    border-color: color-mix(in srgb, #16a34a 45%, var(--border-color));
    color: #16a34a;
  }
  .badge.warn {
    border-color: color-mix(in srgb, #f59e0b 45%, var(--border-color));
    color: #f59e0b;
  }
  .badge.danger {
    border-color: color-mix(in srgb, #ef4444 45%, var(--border-color));
    color: #ef4444;
  }
  .pill {
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-secondary);
    padding: 0.25rem 0.55rem;
    border-radius: 999px;
    font-weight: 800;
  }

  :global([data-theme='light']) .pill {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(0, 0, 0, 0.03);
  }

  .error-line {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: #ef4444;
    font-weight: 750;
    font-size: 12px;
    opacity: 0.95;
    max-width: 520px;
  }

  .error-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .form {
    display: grid;
    gap: 0.9rem;
  }

  .form label {
    display: grid;
    gap: 0.35rem;
  }

  .form label > span {
    color: var(--text-secondary);
    font-weight: 850;
    font-size: 0.78rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .input {
    width: 100%;
    padding: 0.85rem 0.95rem;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-primary);
    outline: none;
    font-weight: 650;
  }

  :global([data-theme='light']) .input {
    background: rgba(0, 0, 0, 0.03);
  }

  .input:focus {
    border-color: rgba(99, 102, 241, 0.55);
    box-shadow: 0 0 0 4px rgba(99, 102, 241, 0.14);
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }

  .grid2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.8rem;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.9rem;
    padding: 0.9rem 1rem;
    border-radius: 14px;
    border: 1px solid var(--border-color);
    background: rgba(255, 255, 255, 0.03);
  }

  :global([data-theme='light']) .toggle-row {
    background: rgba(0, 0, 0, 0.02);
  }

  .toggle-text {
    min-width: 0;
    display: grid;
    gap: 0.15rem;
  }

  .toggle-title {
    color: var(--text-primary);
    font-weight: 900;
  }

  .toggle-sub {
    color: var(--text-secondary);
    font-weight: 650;
    font-size: 0.92rem;
    line-height: 1.35;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
    padding-top: 0.25rem;
  }

  .field-hint {
    margin-top: 6px;
    color: var(--text-secondary);
    font-weight: 600;
    font-size: 0.9rem;
    line-height: 1.35;
  }

  .hint {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-secondary);
    font-weight: 650;
    font-size: 0.9rem;
  }

  .spin {
    display: inline-flex;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 768px) {
    .page-content {
      padding: 16px;
    }

    .error-line {
      max-width: 100%;
    }

    .grid2 {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 1100px) {
  }
</style>
