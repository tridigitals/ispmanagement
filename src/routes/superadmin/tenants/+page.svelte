<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from '$lib/stores/toast';
  import { formatMoney } from '$lib/utils/money';
  import { get } from 'svelte/store';
  import { superadminTenantsCache } from '$lib/stores/superadminTenants';
  import { superadminPlansCache } from '$lib/stores/superadminPlans';
  import { t } from 'svelte-i18n';

  import { loadSuperadminTenantsModules } from './tenantsPageModules';

  let tenants = $state<any[]>([]);
  let plans = $state<any[]>([]);
  let loading = $state(true);
  let isRefreshing = $state(false);
  let error = $state('');
  let isMobile = $state(false);
  let viewMode = $state<'cards' | 'table'>('table');

  // Modal state
  let isEditing = $state(false);
  let editingId = $state('');
  let showCreateModal = $state(false);
  let newTenant = $state({
    name: '',
    slug: '',
    customDomain: '',
    savedCustomDomain: '',
    customDomainStatus: 'none',
    customDomainVerifiedAt: null,
    customDomainFailureReason: null,
    domainStatusReason: '',
    ownerEmail: '',
    ownerPassword: '',
    isActive: true,
    planId: '',
  });
  let creating = $state(false);
  let domainStatusSaving = $state(false);

  // Confirm Dialog State
  let showConfirm = $state(false);
  let confirmLoading = $state(false);
  let pendingDeleteId = $state('');

  // Activate/Deactivate Tenant dialog
  let showToggleConfirm = $state(false);
  let toggleLoading = $state(false);
  let pendingToggleTenant = $state<any | null>(null);

  let searchQuery = $state('');
  let statusFilter = $state<'all' | 'active' | 'inactive'>('all');
  let filtersOpen = $state(false);

  let TenantTableComponent = $state<any>(null);
  let TenantFormModalComponent = $state<any>(null);
  let ConfirmDialogComponent = $state<any>(null);
  let modulesLoading = $state(false);

  let stats = $derived({
    total: tenants.length,
    active: tenants.filter((t) => t.is_active).length,
    inactive: tenants.filter((t) => !t.is_active).length,
  });

  let filteredTenants = $derived(
    tenants.filter((t) => {
      const q = searchQuery.trim().toLowerCase();
      const matchesSearch =
        !q ||
        String(t.name || '')
          .toLowerCase()
          .includes(q) ||
        String(t.slug || '')
          .toLowerCase()
          .includes(q) ||
        String(t.custom_domain || '')
          .toLowerCase()
          .includes(q);

      const matchesStatus =
        statusFilter === 'all' || (statusFilter === 'active' ? t.is_active : !t.is_active);

      return matchesSearch && matchesStatus;
    }),
  );

  // Table columns
  const columns = [
    { key: 'name', label: 'Tenant Name', sortable: true },
    { key: 'slug', label: 'Slug', sortable: true },
    { key: 'custom_domain', label: 'Custom Domain', sortable: true },
    { key: 'is_active', label: 'Status', sortable: true },
    { key: 'created_at', label: 'Created At', sortable: true },
    { key: 'actions', label: 'Actions', align: 'right' },
  ];

  onMount(() => {
    let cleanup: (() => void) | undefined;

    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 899px)');
      const sync = () => {
        isMobile = mq.matches;
        if (mq.matches) viewMode = 'cards';
      };
      sync();

      try {
        mq.addEventListener('change', sync);
        cleanup = () => mq.removeEventListener('change', sync);
      } catch {
        // Safari/older WebView fallback
        // @ts-ignore
        mq.addListener?.(sync);
        // @ts-ignore
        cleanup = () => mq.removeListener?.(sync);
      }
    }

    const cachedTenants = get(superadminTenantsCache);
    if (cachedTenants?.fetchedAt && cachedTenants.tenants?.length) {
      tenants = cachedTenants.tenants as any[];
      loading = false;
      void loadData({ silent: true });
    } else {
      void loadData();
    }

    void ensureTenantsModulesLoaded();

    return cleanup;
  });

  $effect(() => {
    if (isMobile && viewMode === 'table') viewMode = 'cards';
  });

  async function ensureTenantsModulesLoaded() {
    if ((TenantTableComponent && TenantFormModalComponent && ConfirmDialogComponent) || modulesLoading)
      return;

    modulesLoading = true;
    try {
      const {
        TenantTableComponent: TenantTable,
        TenantFormModalComponent: TenantFormModal,
        ConfirmDialogComponent: ConfirmDialog,
      } = await loadSuperadminTenantsModules();
      TenantTableComponent = TenantTable;
      TenantFormModalComponent = TenantFormModal;
      ConfirmDialogComponent = ConfirmDialog;
    } finally {
      modulesLoading = false;
    }
  }

  function mapPlansToSelect(plansRes: any[]) {
    plans = (plansRes || [])
      .filter((p) => p.is_active)
      .map((p) => ({
        label: `${p.name} - ${p.price_monthly > 0 ? `${formatMoney(p.price_monthly)}/mo` : 'Free'}`,
        value: p.id,
      }));

    const defaultPlan = (plansRes || []).find((p) => p.is_default);
    if (defaultPlan) {
      newTenant.planId = defaultPlan.id;
    } else if (!newTenant.planId && plans.length > 0) {
      newTenant.planId = plans[0].value;
    }
  }

  async function loadData(opts: { silent?: boolean } = {}) {
    if (opts.silent) isRefreshing = true;
    else loading = true;
    try {
      const cachedPlans = get(superadminPlansCache);
      if (cachedPlans?.fetchedAt && cachedPlans.plans?.length) {
        mapPlansToSelect(cachedPlans.plans as any[]);
      }

      const [tenantsRes, plansRes] = await Promise.all([
        api.superadmin.listTenants(),
        api.plans.list().catch(() => null),
      ]);

      if (Array.isArray(tenantsRes)) {
        tenants = tenantsRes;
      } else if (tenantsRes && Array.isArray(tenantsRes.data)) {
        tenants = tenantsRes.data;
      } else {
        tenants = [];
      }

      superadminTenantsCache.set({ tenants, fetchedAt: Date.now() });

      if (plansRes) {
        mapPlansToSelect(plansRes as any[]);
        superadminPlansCache.set({
          plans: plansRes as any[],
          fetchedAt: Date.now(),
        });
      }
    } catch (e: any) {
      console.error('Load data error:', e);
      error = e.toString();
      if (e.toString().includes('Unauthorized')) {
        goto('/dashboard');
      }
    } finally {
      loading = false;
      isRefreshing = false;
    }
  }

  async function loadTenants() {
    try {
      const res: any = await api.superadmin.listTenants();
      if (Array.isArray(res)) {
        tenants = res;
      } else if (res && Array.isArray(res.data)) {
        tenants = res.data;
      }
      superadminTenantsCache.set({ tenants, fetchedAt: Date.now() });
    } catch (e) {
      console.error('Reload error', e);
    }
  }

  function resetFilters() {
    searchQuery = '';
    statusFilter = 'all';
  }

  async function openCreateModal() {
    await ensureTenantsModulesLoaded();
    isEditing = false;
    editingId = '';

    Object.assign(newTenant, {
      name: '',
      slug: '',
      customDomain: '',
      savedCustomDomain: '',
      customDomainStatus: 'none',
      customDomainVerifiedAt: null,
      customDomainFailureReason: null,
      domainStatusReason: '',
      ownerEmail: '',
      ownerPassword: '',
      isActive: true,
      planId: plans.length > 0 ? plans[0].value : '',
    });
    showCreateModal = true;
  }

  async function openEditModal(tenant: any) {
    await ensureTenantsModulesLoaded();
    isEditing = true;
    editingId = tenant.id;
    Object.assign(newTenant, {
      name: tenant.name,
      slug: tenant.slug,
      customDomain: tenant.custom_domain || '',
      savedCustomDomain: tenant.custom_domain || '',
      customDomainStatus: tenant.custom_domain_status || 'none',
      customDomainVerifiedAt: tenant.custom_domain_verified_at || null,
      customDomainFailureReason: tenant.custom_domain_failure_reason || null,
      domainStatusReason: tenant.custom_domain_failure_reason || '',
      ownerEmail: '---', // Email cannot be changed here easily in this view
      ownerPassword: '', // Password not needed for update
      isActive: tenant.is_active,
      planId: tenant.plan_id || '', // Include planId from tenant record
    });
    showCreateModal = true;
  }

  async function handleSubmit() {
    if (isEditing) {
      await updateTenant();
    } else {
      await createTenant();
    }
  }

  async function applyDomainStatus() {
    if (!isEditing || !editingId || !newTenant.customDomain) return;

    domainStatusSaving = true;
    try {
      const nextStatus = (
        ['pending', 'active', 'failed'].includes(String(newTenant.customDomainStatus))
          ? newTenant.customDomainStatus
          : 'pending'
      ) as 'pending' | 'active' | 'failed';

      const updated = await api.superadmin.updateTenantDomainStatus(
        editingId,
        nextStatus,
        nextStatus === 'failed' ? newTenant.domainStatusReason || null : null,
      );

      Object.assign(newTenant, {
        customDomainStatus: updated.custom_domain_status || 'none',
        customDomainVerifiedAt: updated.custom_domain_verified_at || null,
        customDomainFailureReason: updated.custom_domain_failure_reason || null,
        domainStatusReason: updated.custom_domain_failure_reason || '',
      });

      toast.success('Status domain berhasil diperbarui');
      await loadTenants();
    } catch (e: any) {
      toast.error(e?.message || 'Gagal memperbarui status domain');
    } finally {
      domainStatusSaving = false;
    }
  }

  async function updateTenant() {
    if (!newTenant.name || !newTenant.slug) return;
    creating = true;
    try {
      await api.superadmin.updateTenant(
        editingId,
        newTenant.name,
        newTenant.slug,
        newTenant.customDomain || null,
        newTenant.isActive,
        newTenant.planId || undefined,
      );
      showCreateModal = false;
      toast.success(get(t)('superadmin.tenants.toasts.updated') || 'Tenant updated successfully');
      await loadTenants();
    } catch (e: any) {
      toast.error(
        get(t)('superadmin.tenants.toasts.update_failed', {
          values: { message: e?.message || e },
        }) || 'Failed to update tenant: ' + e,
      );
    } finally {
      creating = false;
    }
  }

  async function createTenant() {
    if (!newTenant.name || !newTenant.slug || !newTenant.ownerEmail || !newTenant.ownerPassword)
      return;
    creating = true;
    try {
      await api.superadmin.createTenant(
        newTenant.name,
        newTenant.slug,
        newTenant.customDomain || null,
        newTenant.ownerEmail,
        newTenant.ownerPassword,
        newTenant.planId || undefined, // Pass planId
      );

      showCreateModal = false;
      toast.success(get(t)('superadmin.tenants.toasts.created') || 'Tenant created successfully');
      await loadTenants();
    } catch (e: any) {
      toast.error(
        get(t)('superadmin.tenants.toasts.create_failed', {
          values: { message: e?.message || e },
        }) || 'Failed to create tenant: ' + e,
      );
    } finally {
      creating = false;
    }
  }

  function generateSlug() {
    if (!newTenant.name) return;
    newTenant.slug = newTenant.name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/(^-|-$)/g, '');
  }

  async function confirmDelete(id: string) {
    await ensureTenantsModulesLoaded();
    pendingDeleteId = id;
    showConfirm = true;
  }

  async function confirmToggleTenant(tenant: any) {
    await ensureTenantsModulesLoaded();
    pendingToggleTenant = tenant;
    showToggleConfirm = true;
  }

  async function handleDelete() {
    if (!pendingDeleteId) return;
    confirmLoading = true;
    try {
      await api.superadmin.deleteTenant(pendingDeleteId);
      toast.success(get(t)('superadmin.tenants.toasts.deleted') || 'Tenant deleted successfully');
      showConfirm = false;
      await loadTenants();
    } catch (e: any) {
      toast.error(
        get(t)('superadmin.tenants.toasts.delete_failed', {
          values: { message: e?.message || e },
        }) || 'Failed to delete tenant: ' + e,
      );
    } finally {
      confirmLoading = false;
      pendingDeleteId = '';
    }
  }

  let toggleKeyword = $derived.by(() =>
    pendingToggleTenant?.is_active ? 'DEACTIVATE' : 'ACTIVATE',
  );

  let toggleTitle = $derived.by(() =>
    pendingToggleTenant?.is_active
      ? $t('superadmin.tenants.toggle.deactivate_title') || 'Deactivate Tenant'
      : $t('superadmin.tenants.toggle.activate_title') || 'Activate Tenant',
  );

  let toggleType = $derived.by((): 'danger' | 'warning' | 'info' =>
    pendingToggleTenant?.is_active ? 'danger' : 'info',
  );

  let toggleMessage = $derived.by(() => {
    const name =
      pendingToggleTenant?.name || $t('superadmin.tenants.toggle.this_tenant') || 'this tenant';
    if (pendingToggleTenant?.is_active) {
      return (
        $t('superadmin.tenants.toggle.deactivate_message', {
          values: { name },
        }) || `Deactivate ${name}? Users in this tenant will be blocked from accessing the app.`
      );
    }
    return (
      $t('superadmin.tenants.toggle.activate_message', {
        values: { name },
      }) || `Activate ${name}? Users in this tenant will regain access.`
    );
  });

  let toggleConfirmText = $derived.by(() =>
    pendingToggleTenant?.is_active
      ? $t('superadmin.tenants.actions.deactivate') || 'Deactivate'
      : $t('superadmin.tenants.actions.activate') || 'Activate',
  );

  async function handleToggleTenant() {
    if (!pendingToggleTenant) return;
    toggleLoading = true;
    try {
      await api.superadmin.updateTenant(
        pendingToggleTenant.id,
        pendingToggleTenant.name,
        pendingToggleTenant.slug,
        pendingToggleTenant.custom_domain || null,
        !pendingToggleTenant.is_active,
      );
      toast.success(
        pendingToggleTenant.is_active
          ? get(t)('superadmin.tenants.toasts.deactivated') || 'Tenant deactivated'
          : get(t)('superadmin.tenants.toasts.activated') || 'Tenant activated',
      );
      showToggleConfirm = false;
      pendingToggleTenant = null;
      await loadTenants();
    } catch (e: any) {
      toast.error(
        get(t)('superadmin.tenants.toasts.update_failed', {
          values: { message: e?.message || e },
        }) || 'Failed to update tenant: ' + e,
      );
    } finally {
      toggleLoading = false;
    }
  }
</script>

<div class="sa-tenants fade-in">
  <!-- ── Page header ── -->
  <div class="page-head">
    <div>
      <div class="crumbs">
        {$t('superadmin.tenants.crumbs.root')}
        <span class="crumb-sep">›</span>
        <b>{$t('superadmin.tenants.crumbs.tenants')}</b>
      </div>
      <h1>{$t('superadmin.tenants.title')}</h1>
      <p class="subtitle">{$t('superadmin.tenants.subtitle')}</p>
    </div>
    <div class="head-actions">
      {#if isRefreshing}
        <span class="refresh-pill" title={$t('superadmin.tenants.refreshing_title')}>
          <span class="spinner-xs"></span>
          {$t('superadmin.tenants.refreshing')}
        </span>
      {/if}
      <button class="btn ghost" onclick={() => loadData()}><Icon name="refresh-cw" size={14} /> {$t('common.refresh')}</button>
      <button class="btn primary" onclick={openCreateModal}><Icon name="plus" size={14} /> {$t('superadmin.tenants.actions.new')}</button>
    </div>
  </div>

  <!-- ── Stat strip / filter chips ── -->
  <div class="stats-row" aria-label={$t('superadmin.tenants.aria.stats')}>
    <button class="stat-chip" class:on={statusFilter === 'all'} onclick={() => (statusFilter = 'all')}>
      <span class="chip-val">{stats.total}</span>
      <span class="chip-lbl">{$t('superadmin.tenants.stats.all_title')}</span>
    </button>
    <button class="stat-chip" class:on={statusFilter === 'active'} onclick={() => (statusFilter = 'active')}>
      <span class="chip-val">{stats.active}</span>
      <span class="chip-lbl">{$t('superadmin.tenants.stats.active_title')}</span>
    </button>
    <button class="stat-chip" class:on={statusFilter === 'inactive'} onclick={() => (statusFilter = 'inactive')}>
      <span class="chip-val">{stats.inactive}</span>
      <span class="chip-lbl">{$t('superadmin.tenants.stats.inactive_title')}</span>
    </button>
  </div>

  <!-- ── Table panel ── -->
  <div class="panel">
    <div class="panel-head">
      <div class="search-wrap">
        <span class="search-icon"><Icon name="search" size={14} /></span>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder={$t('superadmin.tenants.search')}
          class="search-input"
        />
      </div>
      <div class="panel-tools">
        <button class="icon-btn small" class:active={viewMode === 'cards'} onclick={() => (viewMode = 'cards')} title={$t('common.cards') || 'Cards'}>
          <Icon name="layout-grid" size={15} />
        </button>
        <button class="icon-btn small" class:active={viewMode === 'table'} onclick={() => (viewMode = 'table')} title={$t('common.table') || 'Table'}>
          <Icon name="list" size={15} />
        </button>
      </div>
    </div>

    {#if error}
      <div class="error-state">
        <Icon name="alert-circle" size={48} class="error-icon" />
        <p>{error}</p>
        <button class="btn btn-secondary" onclick={() => loadData()}>
          {$t('common.retry')}
        </button>
      </div>
    {:else}
      {#if TenantTableComponent}
        <TenantTableComponent
          tenants={filteredTenants}
          {loading}
          {viewMode}
          {isMobile}
          {columns}
          onEdit={openEditModal}
          onDelete={(id: string) => confirmDelete(id)}
          onToggleStatus={confirmToggleTenant}
        />
      {:else}
        <div class="error-state">
          <Icon name="refresh-cw" size={28} class="spin" />
          <p>{$t('superadmin.tenants.loading')}</p>
        </div>
      {/if}
    {/if}
  </div>
</div>

{#if TenantFormModalComponent}
      <TenantFormModalComponent
        bind:show={showCreateModal}
        {isEditing}
        bind:newTenant
        {plans}
        loading={creating}
        domainStatusLoading={domainStatusSaving}
        onSubmit={handleSubmit}
        onGenerateSlug={generateSlug}
        onApplyDomainStatus={applyDomainStatus}
      />
{/if}

{#if ConfirmDialogComponent}
  <ConfirmDialogComponent
    bind:show={showConfirm}
    title={$t('superadmin.tenants.delete.title')}
    message={$t('superadmin.tenants.delete.message')}
    confirmText={$t('superadmin.tenants.delete.confirm')}
    confirmationKeyword="DELETE"
    type="danger"
    loading={confirmLoading}
    onconfirm={handleDelete}
  />

  <ConfirmDialogComponent
    bind:show={showToggleConfirm}
    title={toggleTitle}
    message={toggleMessage}
    confirmText={toggleConfirmText}
    confirmationKeyword={toggleKeyword}
    type={toggleType}
    loading={toggleLoading}
    onconfirm={handleToggleTenant}
  />
{/if}

<style>
  .sa-tenants {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1280px;
    margin: 0 auto;
    color: var(--text-primary);
  }

  /* ── Page header (shared across all superadmin pages) ── */
  .page-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 22px;
    flex-wrap: wrap;
  }

  .crumbs {
    font-size: 0.75rem;
    color: var(--text-secondary);
    opacity: 0.75;
    margin-bottom: 6px;
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .crumbs b { font-weight: 500; opacity: 1; }

  .page-head h1 {
    font-size: 1.45rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    margin: 0;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 0.875rem;
    margin: 2px 0 0;
  }

  .head-actions {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    transition: all 0.15s;
  }

  .btn:hover { border-color: var(--color-primary); }

  .btn.primary {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: #0b0d14;
  }

  .btn.primary:hover { filter: brightness(1.1); }
  .btn.ghost { background: transparent; }

  .refresh-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0.75rem;
    border-radius: 99px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    font-weight: 750;
    font-size: 0.82rem;
  }

  .spinner-xs {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  /* ── Stat chips (horizontal clickable filter row) ── */
  .stats-row {
    display: flex;
    gap: 12px;
    margin-bottom: 20px;
    flex-wrap: wrap;
  }

  .stat-chip {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 20px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    cursor: pointer;
    transition: all 0.15s;
    color: var(--text-primary);
    text-align: left;
    min-width: 100px;
  }

  .stat-chip.on {
    border-color: var(--color-primary);
    background: rgba(139, 156, 255, 0.06);
  }

  .stat-chip:hover {
    border-color: var(--color-primary);
    transform: translateY(-1px);
  }

  .chip-val {
    font-size: 1.4rem;
    font-weight: 800;
    letter-spacing: -0.02em;
  }

  .chip-lbl {
    font-size: 0.75rem;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  /* ── Table panel ── */
  .panel {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    border-bottom: 1px solid var(--border-color);
    gap: 12px;
  }

  .search-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-raised);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 6px 12px;
    flex: 1;
    max-width: 360px;
  }

  .search-icon {
    color: var(--text-secondary);
    display: flex;
  }

  .search-input {
    border: none;
    background: none;
    outline: none;
    color: var(--text-primary);
    font-size: 0.85rem;
    width: 100%;
  }

  .search-input::placeholder { color: var(--text-secondary); }

  .panel-tools {
    display: flex;
    gap: 6px;
  }

  .icon-btn.small {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    border: 1px solid var(--border-color);
    background: var(--bg-raised);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }

  .icon-btn.small:hover { color: var(--color-primary); border-color: var(--color-primary); }

  .icon-btn.small.active {
    background: rgba(139, 156, 255, 0.1);
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  /* Error / loading states */
  .error-state {
    padding: 2rem 1.25rem;
    text-align: center;
    color: var(--text-secondary);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .btn-secondary:hover { background: var(--bg-hover); }

  @media (max-width: 768px) {
    .page-head { align-items: flex-start; flex-direction: column; }
    .stat-chip { min-width: 80px; padding: 10px 14px; }
  }
</style>
