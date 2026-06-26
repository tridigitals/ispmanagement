<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { goto } from '$app/navigation';
  import { fly } from 'svelte/transition';
  import Icon from '$lib/components/ui/Icon.svelte';
  import CompactFilterToolbar from '$lib/components/superadmin/shared/CompactFilterToolbar.svelte';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
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
      planId: '', // Plan cannot be changed here for now (use subscription page)
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

<div class="superadmin-content fade-in">
  <div class="stats-row" aria-label={$t('superadmin.tenants.aria.stats')}>
    <button
      class="stat-btn"
      class:active={statusFilter === 'all'}
      onclick={() => (statusFilter = 'all')}
      aria-label={$t('superadmin.tenants.stats.show_all')}
      title={$t('superadmin.tenants.stats.show_all')}
      type="button"
    >
      <StatsCard
        title={$t('superadmin.tenants.stats.all_title')}
        value={stats.total}
        icon="database"
        color="primary"
      />
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'active'}
      onclick={() => (statusFilter = 'active')}
      aria-label={$t('superadmin.tenants.stats.show_active')}
      title={$t('superadmin.tenants.stats.show_active')}
      type="button"
    >
      <StatsCard
        title={$t('superadmin.tenants.stats.active_title')}
        value={stats.active}
        icon="check-circle"
        color="success"
      />
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'inactive'}
      onclick={() => (statusFilter = 'inactive')}
      aria-label={$t('superadmin.tenants.stats.show_inactive')}
      title={$t('superadmin.tenants.stats.show_inactive')}
      type="button"
    >
      <StatsCard
        title={$t('superadmin.tenants.stats.inactive_title')}
        value={stats.inactive}
        icon="slash"
        color="warning"
      />
    </button>
  </div>

  <div class="glass-card" in:fly={{ y: 20, delay: 80 }}>
    <div class="card-header glass">
      <div>
        <h3>{$t('superadmin.tenants.title')}</h3>
        <span class="muted"
          >{$t('superadmin.tenants.subtitle')}</span
        >
      </div>
      <div class="header-actions">
        {#if isRefreshing}
          <span
            class="refresh-pill"
            title={$t('superadmin.tenants.refreshing_title')}
          >
            <span class="spinner-xs"></span>
            {$t('superadmin.tenants.refreshing')}
          </span>
        {/if}
        <span class="count-badge"
          >{$t('superadmin.tenants.count', {
            values: { count: stats.total },
          }) || `${stats.total} tenants`}</span
        >
      </div>
    </div>

    <div class="toolbar-wrapper">
      <CompactFilterToolbar
        bind:searchQuery
        placeholder={$t('superadmin.tenants.search')}
        bind:filterPanelOpen={filtersOpen}
        activeFilterCount={statusFilter === 'all' ? 0 : 1}
        onReset={resetFilters}
        {isMobile}
        bind:viewMode
      >
        {#snippet advancedFilters()}
          <div class="toolbar-field">
            <label for="tenant-status-filter">
              {$t('superadmin.tenants.filters.status')}
            </label>
            <select id="tenant-status-filter" bind:value={statusFilter}>
              <option value="all">{$t('superadmin.tenants.filters.all') || $t('common.all') || 'All'}</option>
              <option value="active">{$t('superadmin.tenants.filters.active') || $t('common.active') || 'Active'}</option>
              <option value="inactive">{$t('superadmin.tenants.filters.inactive') || $t('common.inactive') || 'Inactive'}</option>
            </select>
          </div>
        {/snippet}
        {#snippet actions()}
          <button class="btn btn-primary" onclick={openCreateModal}>
            <Icon name="plus" size={18} />
            <span>
              {$t('superadmin.tenants.actions.new')}
            </span>
          </button>
        {/snippet}
      </CompactFilterToolbar>
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
  .superadmin-content {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1400px;
    margin: 0 auto;
    color: var(--text-primary);
  }

  .stats-row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .stat-btn {
    border: none;
    padding: 0;
    background: transparent;
    cursor: pointer;
    text-align: left;
    border-radius: var(--radius-lg);
    transition: transform 0.15s ease;
  }

  .stat-btn:hover {
    transform: translateY(-1px);
  }

  .stat-btn.active :global(.stats-card) {
    border-color: color-mix(in srgb, var(--color-primary) 35%, var(--border-color));
    box-shadow: 0 0 0 1px var(--color-primary-subtle);
  }

  .glass-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
  }

  :global([data-theme='light']) .glass-card {
    background: var(--bg-surface);
    border-color: var(--border-color);
    box-shadow: var(--shadow-sm);
  }

  .card-header {
    padding: 1.25rem 1.25rem 1rem 1.25rem;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .header-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .refresh-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    font-weight: 750;
    font-size: 0.85rem;
    user-select: none;
  }

  .spinner-xs {
    width: 14px;
    height: 14px;
    border-radius: 999px;
    border: 2px solid var(--border-color);
    border-top-color: var(--color-primary);
    animation: spin 0.9s linear infinite;
  }

  :global([data-theme='light']) .card-header {
    border-bottom-color: var(--border-color);
  }

  .card-header h3 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: -0.01em;
  }

  .muted {
    display: block;
    margin-top: 0.25rem;
    color: var(--text-secondary);
    font-size: 0.92rem;
  }

  .count-badge {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    font-size: 0.85rem;
    font-weight: 650;
    white-space: nowrap;
    align-self: flex-start;
  }

  :global([data-theme='light']) .count-badge {
    background: var(--bg-tertiary);
    border-color: var(--border-color);
  }

  .toolbar-wrapper {
    padding: 1rem 1.25rem 0.25rem 1.25rem;
  }

  .toolbar-field {
    display: grid;
    gap: 0.32rem;
    max-width: 240px;
  }

  .toolbar-field label {
    font-size: 0.74rem;
    font-weight: 800;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .toolbar-field select {
    min-height: 40px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    padding: 0 0.75rem;
    outline: none;
  }

  .toolbar-field select:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px var(--color-primary-subtle);
  }

  .error-state {
    padding: 2rem 1.25rem;
    text-align: center;
    color: var(--text-secondary);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
  }

  :global(.error-icon) {
    color: var(--color-danger);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.25rem;
    border-radius: var(--radius-md);
    font-weight: 600;
    font-size: 0.9rem;
    border: none;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary {
    background: var(--color-primary);
    color: var(--bg-app);
  }

  .btn-primary:hover {
    background: var(--color-primary-hover);
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .btn-secondary:hover {
    background: var(--bg-hover);
  }

  @media (max-width: 768px) {
    .stats-row {
      grid-template-columns: 1fr;
      gap: 0.75rem;
    }

    .toolbar-wrapper {
      padding: 0.9rem 1rem 0 1rem;
    }

    .btn {
      justify-content: center;
    }
  }
</style>
