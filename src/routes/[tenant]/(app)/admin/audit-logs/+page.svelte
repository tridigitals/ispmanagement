<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { can, isAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import type { AuditLog } from '$lib/api/client';
  import { t } from 'svelte-i18n';
  import Icon from '$lib/components/ui/Icon.svelte';

  import AuditLogFilters from '$lib/components/superadmin/audit-logs/AuditLogFilters.svelte';
  import AuditLogTable from '$lib/components/superadmin/audit-logs/AuditLogTable.svelte';

  let logs = $state<AuditLog[]>([]);
  let loading = $state(true);
  let page = $state(1);
  let total = $state(0);
  let pageSize = $state(20);
  let isMobile = $state(false);
  let viewMode = $state<'table' | 'cards'>('table');
  let errorMessage = $state<string | null>(null);

  // Filters
  let searchQuery = $state('');
  let actionFilter = $state('');
  let dateFrom = $state('');
  let dateTo = $state('');
  let userIdFilter = $state('');

  const canRead = $derived($can('read', 'audit_logs'));

  let searchTimer: any;
  function handleSearch() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      page = 1;
      loadLogs();
    }, 500);
  }

  function clearFilters() {
    searchQuery = '';
    actionFilter = '';
    dateFrom = '';
    dateTo = '';
    userIdFilter = '';
    page = 1;
    void loadLogs();
  }

  async function loadLogs() {
    if (!canRead) return;

    loading = true;
    errorMessage = null;
    try {
      const activeFilters: any = {};
      if (searchQuery) activeFilters.search = searchQuery;
      if (actionFilter) activeFilters.action = actionFilter;
      if (dateFrom) activeFilters.date_from = new Date(dateFrom).toISOString();
      if (dateTo) activeFilters.date_to = new Date(dateTo).toISOString();
      if (userIdFilter) activeFilters.user_id = userIdFilter;

      const res = await api.audit.listTenant(page, pageSize, activeFilters);
      logs = res.data;
      total = res.total;
    } catch (err: any) {
      const msg = String(err?.message || err || 'Failed to load audit logs');
      errorMessage = msg;
      logs = [];
      total = 0;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    let cleanup: (() => void) | undefined;

    const unsubAdmin = isAdmin.subscribe((value) => {
      if (!value) {
        goto('/dashboard');
      }
    });

    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 899px)');
      const sync = () => (isMobile = mq.matches);
      sync();
      try {
        mq.addEventListener('change', sync);
        cleanup = () => {
          mq.removeEventListener('change', sync);
          unsubAdmin();
        };
      } catch {
        // @ts-ignore
        mq.addListener?.(sync);
        // @ts-ignore
        cleanup = () => {
          mq.removeListener?.(sync);
          unsubAdmin();
        };
      }
    }

    void loadLogs();
    return cleanup;
  });

  $effect(() => {
    if (isMobile) viewMode = 'cards';
  });

  function handlePageChange(newPage: number) {
    page = newPage + 1;
    loadLogs();
  }

  function handlePageSizeChange(newSize: number) {
    pageSize = newSize;
    page = 1;
    loadLogs();
  }
</script>

<div class="admin-content fade-in">
  <div class="glass-card">
    {#if !canRead}
      <div class="empty-state">
        <div class="empty-icon">
          <Icon name="lock" size={22} />
        </div>
        <div class="empty-text">
          <h4>{$t('common.forbidden') || 'Forbidden'}</h4>
          <p>{$t('common.no_permission') || 'You do not have permission to view this page.'}</p>
        </div>
      </div>
    {:else}
      <AuditLogFilters
        bind:searchQuery
        bind:actionFilter
        bind:dateFrom
        bind:dateTo
        bind:viewMode
        {isMobile}
        onSearch={handleSearch}
        onClear={clearFilters}
      />

      {#if errorMessage}
        <div class="error-box" role="alert">
          <div class="error-icon">
            <Icon name="alert-circle" size={18} />
          </div>
          <div class="error-text">
            <div class="error-title">
              {$t('superadmin.audit_logs.title') || 'Audit Logs'}
            </div>
            <div class="error-msg">{errorMessage}</div>
            {#if errorMessage.toLowerCase().includes('upgrade')}
              <button class="btn btn-primary" onclick={() => goto('../subscription')}>
                {$t('admin.subscription.title') || 'Subscription'}
              </button>
            {/if}
          </div>
        </div>
      {/if}

      <AuditLogTable
        {logs}
        {loading}
        {page}
        {total}
        {pageSize}
        {viewMode}
        {isMobile}
        onPageChange={handlePageChange}
        onPageSizeChange={handlePageSizeChange}
      />
    {/if}
  </div>
</div>

<style>
  .admin-content {
    padding: clamp(12px, 2vw, 32px);
    max-width: 1400px;
    margin: 0 auto;
    color: var(--text-primary);
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    min-width: 0;
    --glass: rgba(255, 255, 255, 0.04);
    --glass-border: rgba(255, 255, 255, 0.08);
  }

  .glass-card {
    background: linear-gradient(145deg, var(--bg-surface), #0b0c10);
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.28);
    overflow: hidden;
  }

  :global([data-theme='light']) .glass-card {
    background: linear-gradient(135deg, #ffffff, #f7f7fb);
    border-color: rgba(0, 0, 0, 0.06);
    box-shadow:
      0 12px 32px rgba(0, 0, 0, 0.08),
      0 0 0 1px rgba(255, 255, 255, 0.8);
  }

  .empty-state {
    padding: 2.5rem 2rem;
    display: grid;
    gap: 1rem;
    text-align: center;
    color: var(--text-secondary);
  }

  .empty-icon {
    width: 56px;
    height: 56px;
    border-radius: 14px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto;
    background: rgba(99, 102, 241, 0.12);
    color: var(--color-primary);
    border: 1px solid rgba(99, 102, 241, 0.35);
  }

  .empty-text h4 {
    margin: 0;
    font-size: 1.05rem;
    color: var(--text-primary);
  }

  .empty-text p {
    margin: 0.35rem 0 0;
    font-size: 0.9rem;
  }

  .error-box {
    margin: 0 1rem 1rem;
    padding: 0.9rem 1rem;
    border-radius: 12px;
    border: 1px solid rgba(239, 68, 68, 0.25);
    background: rgba(239, 68, 68, 0.08);
    display: flex;
    gap: 0.75rem;
    align-items: flex-start;
  }

  .error-icon {
    padding-top: 2px;
    color: var(--color-danger);
  }

  .error-title {
    font-weight: 800;
    color: var(--text-primary);
    margin-bottom: 0.15rem;
  }

  .error-msg {
    color: var(--text-secondary);
    font-size: 0.9rem;
    margin-bottom: 0.65rem;
    word-break: break-word;
  }
</style>
