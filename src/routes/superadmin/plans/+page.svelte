<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import CompactFilterToolbar from '$lib/components/superadmin/shared/CompactFilterToolbar.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import StatsCard from '$lib/components/dashboard/StatsCard.svelte';
  import { toast } from '$lib/stores/toast';
  import { formatMoney } from '$lib/utils/money';
  import { superadminPlansCache, type SuperadminPlan } from '$lib/stores/superadminPlans';
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import { extractApiErrorMessage } from '$lib/api/core';

  let plans = $state<SuperadminPlan[]>([]);
  let loading = $state(true);
  let isRefreshing = $state(false);

  // Confirm Dialog State
  let showConfirm = $state(false);
  let confirmLoading = $state(false);
  let confirmTitle = $state('');
  let confirmMessage = $state('');
  let confirmAction = $state<(() => Promise<void>) | null>(null);
  let confirmType = $state<'danger' | 'warning' | 'info'>('danger');
  let confirmKeyword = $state('');

  // Table configuration
  const planColumns = [
    { key: 'name', label: 'Plan', width: '30%' },
    { key: 'pricing', label: 'Pricing', width: '20%' },
    { key: 'status', label: 'Status', width: '18%' },
    { key: 'sort_order', label: 'Order', width: '10%' },
    {
      key: 'actions',
      label: 'Actions',
      width: '22%',
      align: 'right' as const,
    },
  ];

  let planSearch = $state('');
  let statusFilter = $state<'all' | 'active' | 'inactive'>('all');
  let viewMode = $state<'cards' | 'table'>('cards');
  let isMobile = $state(false);
  let filtersOpen = $state(false);

  let stats = $derived({
    total: plans.length,
    active: plans.filter((p) => p.is_active).length,
    inactive: plans.filter((p) => !p.is_active).length,
    defaultPlan: plans.find((p) => p.is_default) || null,
  });

  let filteredPlans = $derived(
    plans
      .filter((p) => {
        const q = planSearch.trim().toLowerCase();
        const matchesSearch =
          !q || p.name.toLowerCase().includes(q) || p.slug.toLowerCase().includes(q);

        const matchesStatus =
          statusFilter === 'all' || (statusFilter === 'active' ? p.is_active : !p.is_active);

        return matchesSearch && matchesStatus;
      })
      .sort((a, b) => (a.sort_order ?? 0) - (b.sort_order ?? 0)),
  );

  onMount(() => {
    let cleanup: (() => void) | undefined;

    const cached = get(superadminPlansCache);
    if (cached?.fetchedAt && cached.plans?.length) {
      plans = cached.plans;
      loading = false;
      void loadData({ silent: true });
    } else {
      void loadData();
    }

    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 720px)');
      const sync = () => {
        isMobile = mq.matches;
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

    return cleanup;
  });

  $effect(() => {
    if (isMobile && viewMode === 'table') viewMode = 'cards';
  });

  async function loadData(opts: { silent?: boolean } = {}) {
    if (opts.silent) isRefreshing = true;
    else loading = true;
    try {
      plans = await api.plans.list();
      superadminPlansCache.set({ plans, fetchedAt: Date.now() });
    } catch (e: any) {
      toast.error(
        extractApiErrorMessage(e, get(t)('superadmin.plans.errors.load_failed') || 'Failed to load data'),
      );
    } finally {
      loading = false;
      isRefreshing = false;
    }
  }

  function createPlan() {
    goto('/superadmin/plans/new');
  }

  function resetFilters() {
    planSearch = '';
    statusFilter = 'all';
  }

  function editPlan(plan: SuperadminPlan) {
    goto(`/superadmin/plans/${plan.id}`);
  }

  // Confirm Dialog Logic
  function openConfirmDialog(
    title: string,
    message: string,
    action: () => Promise<void>,
    type: 'danger' | 'warning' | 'info' = 'danger',
    keyword = '',
  ) {
    confirmTitle = title;
    confirmMessage = message;
    confirmAction = action;
    confirmType = type;
    confirmKeyword = keyword;
    confirmLoading = false;
    showConfirm = true;
  }

  async function handleConfirm() {
    if (!confirmAction) return;
    confirmLoading = true;
    try {
      await confirmAction();
      showConfirm = false;
    } finally {
      confirmLoading = false;
      confirmAction = null;
    }
  }

  function handleCancelConfirm() {
    showConfirm = false;
    confirmLoading = false;
    confirmAction = null;
    confirmKeyword = '';
  }

  function confirmDeletePlan(plan: SuperadminPlan) {
    openConfirmDialog(
      get(t)('superadmin.plans.confirm.delete_title') || 'Delete Plan',
      get(t)('superadmin.plans.confirm.delete_message', {
        values: { name: plan.name },
      }) || `Delete "${plan.name}"? This action cannot be undone.`,
      async () => {
        await api.plans.delete(plan.id);
        toast.success(get(t)('superadmin.plans.toasts.deleted') || 'Plan deleted');
        await loadData();
      },
      'danger',
      'DELETE',
    );
  }

  function confirmToggleActive(plan: SuperadminPlan) {
    if (plan.is_default && plan.is_active) {
      toast.error(
        get(t)('superadmin.plans.errors.default_cannot_deactivate') ||
          'Default plan cannot be deactivated. Set another default first.',
      );
      return;
    }

    const next = !plan.is_active;
    openConfirmDialog(
      next
        ? get(t)('superadmin.plans.confirm.activate_title') || 'Activate Plan'
        : get(t)('superadmin.plans.confirm.deactivate_title') || 'Deactivate Plan',
      next
        ? get(t)('superadmin.plans.confirm.activate_message', {
            values: { name: plan.name },
          }) || `Activate "${plan.name}"? Tenants can be assigned to it again.`
        : get(t)('superadmin.plans.confirm.deactivate_message', {
            values: { name: plan.name },
          }) || `Deactivate "${plan.name}"? Tenants can no longer be assigned to it.`,
      async () => {
        await api.plans.update(
          plan.id,
          plan.name,
          plan.slug,
          plan.description ?? undefined,
          plan.price_monthly,
          plan.price_yearly,
          next,
          plan.is_default,
          plan.sort_order,
        );
        toast.success(
          next
            ? get(t)('superadmin.plans.toasts.activated') || 'Plan activated'
            : get(t)('superadmin.plans.toasts.deactivated') || 'Plan deactivated',
        );
        await loadData();
      },
      next ? 'info' : 'warning',
      next ? 'ACTIVATE' : 'DEACTIVATE',
    );
  }

  function confirmSetDefault(plan: SuperadminPlan) {
    if (plan.is_default) return;

    openConfirmDialog(
      get(t)('superadmin.plans.confirm.default_title') || 'Set Default Plan',
      get(t)('superadmin.plans.confirm.default_message', {
        values: { name: plan.name },
      }) || `Make "${plan.name}" the default plan for new tenants?`,
      async () => {
        const currentDefault = plans.find((p) => p.is_default);
        if (currentDefault && currentDefault.id !== plan.id) {
          await api.plans.update(
            currentDefault.id,
            currentDefault.name,
            currentDefault.slug,
            currentDefault.description ?? undefined,
            currentDefault.price_monthly,
            currentDefault.price_yearly,
            currentDefault.is_active,
            false,
            currentDefault.sort_order,
          );
        }

        await api.plans.update(
          plan.id,
          plan.name,
          plan.slug,
          plan.description ?? undefined,
          plan.price_monthly,
          plan.price_yearly,
          true,
          true,
          plan.sort_order,
        );

        toast.success(get(t)('superadmin.plans.toasts.default_updated') || 'Default plan updated');
        await loadData();
      },
      'info',
      'DEFAULT',
    );
  }

  function formatPrice(price: number): string {
    if (!price || price <= 0) return $t('superadmin.plans.price.free') || 'Free';
    return formatMoney(price);
  }
</script>

<div class="sa-plans fade-in">
  <!-- ── Page header ── -->
  <div class="page-head">
    <div>
      <div class="crumbs">
        {$t('superadmin.plans.crumbs.root')}
        <span class="crumb-sep">›</span>
        <b>{$t('superadmin.plans.crumbs.plans')}</b>
      </div>
      <h1>{$t('superadmin.plans.title')}</h1>
      <p class="subtitle">{$t('superadmin.plans.subtitle')}</p>
    </div>
    <div class="head-actions">
      {#if isRefreshing}
        <span class="refresh-pill" title={$t('superadmin.plans.refreshing_title')}>
          <span class="spinner-xs"></span>
          {$t('superadmin.plans.refreshing')}
        </span>
      {/if}
      <button class="btn ghost" onclick={() => loadData()} type="button"><Icon name="refresh-cw" size={14} /> {$t('common.refresh')}</button>
      <button class="btn primary" onclick={createPlan} type="button">
        <Icon name="plus" size={18} />
        <span>{$t('superadmin.plans.actions.create')}</span>
      </button>
    </div>
  </div>
  <div class="stats-row" aria-label={$t('superadmin.plans.aria.stats')}>
    <button
      class="stat-btn"
      class:active={statusFilter === 'all'}
      onclick={() => (statusFilter = 'all')}
      type="button"
      title={$t('superadmin.plans.stats.show_all')}
    >
      <StatsCard
        title={$t('superadmin.plans.stats.all_title')}
        value={stats.total}
        icon="credit-card"
        color="primary"
      />
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'active'}
      onclick={() => (statusFilter = 'active')}
      type="button"
      title={$t('superadmin.plans.stats.show_active')}
    >
      <StatsCard
        title={$t('superadmin.plans.stats.active_title') || $t('common.active') || 'Active'}
        value={stats.active}
        icon="check-circle"
        color="success"
      />
    </button>
    <button
      class="stat-btn"
      class:active={statusFilter === 'inactive'}
      onclick={() => (statusFilter = 'inactive')}
      type="button"
      title={$t('superadmin.plans.stats.show_inactive')}
    >
      <StatsCard
        title={$t('superadmin.plans.stats.inactive_title') || $t('common.inactive') || 'Inactive'}
        value={stats.inactive}
        icon="ban"
        color="warning"
      />
    </button>
    <button
      class="stat-btn"
      type="button"
      title={$t('superadmin.plans.stats.current_default')}
      disabled={!stats.defaultPlan}
    >
      <StatsCard
        title={$t('superadmin.plans.stats.default_title')}
        value={stats.defaultPlan?.name || '—'}
        icon="star"
        color="primary"
      />
    </button>
  </div>

  <div class="panel">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>{$t('superadmin.plans.loading')}</p>
      </div>
    {:else}
      <div class="toolbar-wrapper">
        <CompactFilterToolbar
          bind:searchQuery={planSearch}
          placeholder={$t('superadmin.plans.search')}
          bind:filterPanelOpen={filtersOpen}
          activeFilterCount={statusFilter === 'all' ? 0 : 1}
          onReset={resetFilters}
          {isMobile}
          bind:viewMode
        >
          {#snippet advancedFilters()}
            <div class="toolbar-field">
              <label for="plan-status-filter">
                {$t('superadmin.plans.filters.status')}
              </label>
              <select id="plan-status-filter" bind:value={statusFilter}>
                <option value="all">{$t('superadmin.plans.filters.all') || $t('common.all') || 'All'}</option>
                <option value="active">{$t('superadmin.plans.filters.active') || $t('common.active') || 'Active'}</option>
                <option value="inactive">{$t('superadmin.plans.filters.inactive') || $t('common.inactive') || 'Inactive'}</option>
              </select>
            </div>
          {/snippet}
        </CompactFilterToolbar>
      </div>

      {#if viewMode === 'cards' || isMobile}
        <div class="plans-grid" aria-label={$t('superadmin.plans.aria.cards')}>
          {#each filteredPlans as plan (plan.id)}
            <div
              class="plan-card"
              onclick={() => editPlan(plan)}
              onkeydown={(e) => e.key === 'Enter' && editPlan(plan)}
              role="button"
              tabindex="0"
            >
              <div class="plan-top">
                <div>
                  <div class="plan-name">
                    <span>{plan.name}</span>
                    {#if plan.is_default}
                      <span
                        class="pill default"
                        title={$t('superadmin.plans.badges.default_title')}
                        >{$t('superadmin.plans.badges.default')}</span
                      >
                    {/if}
                    <span
                      class="pill {plan.is_active ? 'active' : 'inactive'}"
                      title={plan.is_active
                        ? $t('common.active') || 'Active'
                        : $t('common.inactive') || 'Inactive'}
                    >
                      {plan.is_active
                        ? $t('common.active') || 'Active'
                        : $t('common.inactive') || 'Inactive'}
                    </span>
                  </div>
                  <div class="plan-code">{plan.slug}</div>
                </div>
                <div class="plan-price">
                  <div class="price-main">
                    {formatPrice(plan.price_monthly)}<span class="unit"
                      >{$t('common.per_month_short')}</span
                    >
                  </div>
                  <div class="price-sub">
                    {formatPrice(plan.price_yearly)}<span class="unit"
                      >{$t('common.per_year_short')}</span
                    >
                  </div>
                </div>
              </div>

              {#if plan.description}
                <div class="plan-desc">{plan.description}</div>
              {:else}
                <div class="plan-desc muted-text">
                  {$t('superadmin.plans.empty.description')}
                </div>
              {/if}

              <div class="plan-actions">
                <button
                  class="btn-icon"
                  title={$t('common.edit')}
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    editPlan(plan);
                  }}
                >
                  <Icon name="edit" size={18} />
                </button>
                <button
                  class="btn-icon"
                  title={plan.is_default
                    ? $t('superadmin.plans.actions.already_default') || 'Already default'
                    : $t('superadmin.plans.actions.set_default') || 'Set as default'}
                  type="button"
                  disabled={plan.is_default}
                  onclick={(e) => {
                    e.stopPropagation();
                    confirmSetDefault(plan);
                  }}
                >
                  <Icon name="star" size={18} />
                </button>
                <button
                  class="btn-icon {plan.is_active ? 'warn' : 'success'}"
                  title={plan.is_active
                    ? $t('superadmin.plans.actions.deactivate') || 'Deactivate'
                    : $t('superadmin.plans.actions.activate') || 'Activate'}
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    confirmToggleActive(plan);
                  }}
                >
                  <Icon name={plan.is_active ? 'ban' : 'check-circle'} size={18} />
                </button>
                <button
                  class="btn-icon danger"
                  title={$t('common.delete')}
                  type="button"
                  onclick={(e) => {
                    e.stopPropagation();
                    confirmDeletePlan(plan);
                  }}
                >
                  <Icon name="trash" size={18} />
                </button>
              </div>
            </div>
          {/each}

          {#if filteredPlans.length === 0}
            <div class="empty-grid">
              <div class="empty-icon">
                <Icon name="credit-card" size={56} />
              </div>
              <h4>
                {$t('superadmin.plans.empty.title')}
              </h4>
              <p>
                {$t('superadmin.plans.empty.subtitle')}
              </p>
              <button class="btn btn-primary" type="button" onclick={createPlan}>
                <Icon name="plus" size={18} />
                <span>
                  {$t('superadmin.plans.actions.create')}
                </span>
              </button>
            </div>
          {/if}
        </div>
      {:else if viewMode === 'table' && !isMobile}
        <div class="table-wrapper" aria-label={$t('superadmin.plans.aria.table')}>
          <Table
            columns={planColumns}
            data={filteredPlans}
            loading={false}
            keyField="id"
            pagination={true}
            pageSize={10}
            mobileView="scroll"
          >
            {#snippet cell({ item, column, key })}
              {#if key === 'name'}
                <div class="table-plan">
                  <div class="table-plan-name">
                    {item.name}
                  </div>
                  <div class="table-plan-sub">
                    {item.slug}
                  </div>
                </div>
              {:else if key === 'pricing'}
                <div class="table-pricing">
                  <span class="mono">{formatPrice(item.price_monthly)}</span>
                  <span class="sep">/</span>
                  <span class="mono">{formatPrice(item.price_yearly)}</span>
                </div>
              {:else if key === 'status'}
                <div class="status-badges">
                  <span class="badge {item.is_active ? 'success' : 'warning'}">
                    {item.is_active ? 'Active' : 'Inactive'}
                  </span>
                  {#if item.is_default}
                    <span class="badge primary"
                      >{$t('superadmin.plans.badges.default')}</span
                    >
                  {/if}
                </div>
              {:else if key === 'actions'}
                <div class="table-actions">
                  <button
                    class="btn-icon"
                    title={$t('common.edit')}
                    type="button"
                    onclick={() => editPlan(item)}
                  >
                    <Icon name="edit" size={18} />
                  </button>
                  <button
                    class="btn-icon"
                    title={item.is_default
                      ? $t('superadmin.plans.actions.already_default') || 'Already default'
                      : $t('superadmin.plans.actions.set_default') || 'Set as default'}
                    type="button"
                    disabled={item.is_default}
                    onclick={() => confirmSetDefault(item)}
                  >
                    <Icon name="star" size={18} />
                  </button>
                  <button
                    class="btn-icon {item.is_active ? 'warn' : 'success'}"
                    title={item.is_active
                      ? $t('superadmin.plans.actions.deactivate') || 'Deactivate'
                      : $t('superadmin.plans.actions.activate') || 'Activate'}
                    type="button"
                    onclick={() => confirmToggleActive(item)}
                  >
                    <Icon name={item.is_active ? 'ban' : 'check-circle'} size={18} />
                  </button>
                  <button
                    class="btn-icon danger"
                    title={$t('common.delete')}
                    type="button"
                    onclick={() => confirmDeletePlan(item)}
                  >
                    <Icon name="trash" size={18} />
                  </button>
                </div>
              {:else}
                {item[column.key]}
              {/if}
            {/snippet}
          </Table>
        </div>
      {/if}
    {/if}
  </div>
</div>

<ConfirmDialog
  bind:show={showConfirm}
  title={confirmTitle}
  message={confirmMessage}
  type={confirmType}
  confirmationKeyword={confirmKeyword}
  loading={confirmLoading}
  onconfirm={handleConfirm}
  oncancel={handleCancelConfirm}
/>

<style>
  .sa-plans {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1400px;
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

  .stats-row {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
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

  .stat-btn:hover:not(:disabled) {
    transform: translateY(-1px);
  }

  .stat-btn:disabled {
    opacity: 0.7;
    cursor: default;
  }

  .stat-btn.active :global(.stats-card) {
    border-color: rgba(99, 102, 241, 0.35);
    box-shadow: 0 0 0 1px rgba(99, 102, 241, 0.25);
  }

  /* ── Panel (replaces glass-card) ── */
  .panel {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
  }

  .toolbar-wrapper {
    padding: 1rem 1.25rem 0.5rem 1.25rem;
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

  .plans-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1rem;
    padding: 0.75rem 1.25rem 1rem 1.25rem;
  }

  .plan-card {
    border-radius: var(--radius-lg);
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.03);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.25);
    padding: 1rem;
    cursor: pointer;
    transition:
      transform 0.15s ease,
      border-color 0.15s ease,
      background 0.15s ease;
  }

  .plan-card:hover {
    transform: translateY(-1px);
    border-color: rgba(99, 102, 241, 0.25);
    background: rgba(255, 255, 255, 0.04);
  }

  :global([data-theme='light']) .plan-card {
    background: rgba(255, 255, 255, 0.85);
    border-color: rgba(0, 0, 0, 0.06);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.06);
  }

  .plan-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .plan-name {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    font-weight: 800;
    letter-spacing: -0.01em;
    color: var(--text-primary);
    line-height: 1.2;
  }

  .plan-code {
    margin-top: 0.25rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: var(--text-secondary);
    font-size: 0.82rem;
  }

  .plan-price {
    text-align: right;
    white-space: nowrap;
  }

  .price-main {
    font-weight: 900;
    font-size: 1.15rem;
    color: var(--text-primary);
  }

  .price-sub {
    margin-top: 0.1rem;
    font-weight: 650;
    font-size: 0.9rem;
    color: var(--text-secondary);
  }

  .unit {
    font-weight: 650;
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-left: 0.15rem;
  }

  .plan-desc {
    margin-top: 0.85rem;
    color: var(--text-secondary);
    font-size: 0.92rem;
    line-height: 1.5;
    min-height: 2.8em;
  }

  .muted-text {
    opacity: 0.75;
  }

  .plan-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.9rem;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 0.55rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 750;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-primary);
  }

  :global([data-theme='light']) .pill {
    border-color: rgba(0, 0, 0, 0.08);
    background: rgba(0, 0, 0, 0.02);
  }

  .pill.default {
    border-color: rgba(99, 102, 241, 0.35);
    background: rgba(99, 102, 241, 0.12);
  }

  .pill.active {
    border-color: rgba(16, 185, 129, 0.35);
    background: rgba(16, 185, 129, 0.12);
  }

  .pill.inactive {
    border-color: rgba(245, 158, 11, 0.35);
    background: rgba(245, 158, 11, 0.12);
  }

  .table-wrapper {
    padding: 0 1.25rem 1rem 1.25rem;
  }

  .table-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .table-plan-name {
    font-weight: 750;
    color: var(--text-primary);
  }

  .table-plan-sub {
    margin-top: 0.15rem;
    color: var(--text-secondary);
    font-size: 0.82rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  .table-pricing {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    white-space: nowrap;
    color: var(--text-primary);
    font-weight: 650;
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.92rem;
  }

  .sep {
    color: var(--text-secondary);
  }

  .status-badges {
    display: inline-flex;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    padding: 0.22rem 0.6rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-primary);
  }

  :global([data-theme='light']) .badge {
    border-color: rgba(0, 0, 0, 0.08);
    background: rgba(0, 0, 0, 0.02);
  }

  .badge.success {
    border-color: rgba(16, 185, 129, 0.35);
    background: rgba(16, 185, 129, 0.12);
    color: #10b981;
  }

  .badge.warning {
    border-color: rgba(245, 158, 11, 0.35);
    background: rgba(245, 158, 11, 0.12);
    color: #f59e0b;
  }

  .badge.primary {
    border-color: rgba(99, 102, 241, 0.35);
    background: rgba(99, 102, 241, 0.12);
    color: #818cf8;
  }

  .loading-state {
    padding: 3rem 1.25rem;
    display: grid;
    place-items: center;
    gap: 0.75rem;
    color: var(--text-secondary);
  }

  .spinner {
    width: 28px;
    height: 28px;
    border-radius: 999px;
    border: 3px solid rgba(255, 255, 255, 0.12);
    border-top-color: rgba(99, 102, 241, 0.8);
    animation: spin 0.9s linear infinite;
  }

  :global([data-theme='light']) .spinner {
    border-color: rgba(0, 0, 0, 0.08);
    border-top-color: rgba(99, 102, 241, 0.9);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty-grid {
    grid-column: 1 / -1;
    border-radius: var(--radius-lg);
    border: 1px dashed rgba(255, 255, 255, 0.18);
    background: rgba(255, 255, 255, 0.02);
    padding: 2rem 1.25rem;
    text-align: center;
    color: var(--text-secondary);
    display: grid;
    place-items: center;
    gap: 0.5rem;
  }

  :global([data-theme='light']) .empty-grid {
    border-color: rgba(0, 0, 0, 0.12);
    background: rgba(0, 0, 0, 0.01);
  }

  .empty-grid h4 {
    margin: 0.35rem 0 0 0;
    color: var(--text-primary);
    font-size: 1rem;
    font-weight: 800;
  }

  .empty-grid p {
    margin: 0;
    max-width: 46ch;
  }

  .empty-icon {
    color: var(--text-secondary);
    opacity: 0.9;
  }

  :global(.btn-icon.danger:hover:not(:disabled)) {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.35);
    color: #ef4444;
  }

  :global(.btn-icon.warn:hover:not(:disabled)) {
    background: rgba(245, 158, 11, 0.12);
    border-color: rgba(245, 158, 11, 0.35);
    color: #f59e0b;
  }

  :global(.btn-icon.success:hover:not(:disabled)) {
    background: rgba(16, 185, 129, 0.12);
    border-color: rgba(16, 185, 129, 0.35);
    color: #10b981;
  }

  @media (max-width: 768px) {
    .page-head { align-items: flex-start; flex-direction: column; }
  }

  @media (max-width: 1024px) {
    .stats-row {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .plans-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 720px) {
    .plans-grid {
      grid-template-columns: 1fr;
      padding-top: 0.25rem;
    }

    .table-wrapper {
      padding: 0 0.75rem 0.85rem 0.75rem;
    }

    .plan-top {
      flex-direction: column;
      align-items: flex-start;
    }

    .plan-price {
      text-align: left;
    }
  }

  @media (max-width: 480px) {
    .stats-row {
      grid-template-columns: 1fr;
    }
  }
</style>
