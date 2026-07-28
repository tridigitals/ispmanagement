<script lang="ts">
  import { isSuperAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { AuditLog } from '$lib/api/client';

  // New components
  import AuditLogFilters from '$lib/components/superadmin/audit-logs/AuditLogFilters.svelte';
  import AuditLogTable from '$lib/components/superadmin/audit-logs/AuditLogTable.svelte';

  let logs = $state<AuditLog[]>([]);
  let loading = $state(true);
  let page = $state(1);
  let total = $state(0);
  let pageSize = $state(20);
  let isMobile = $state(false);
  let viewMode = $state<'table' | 'cards'>('table');

  // Filters
  let searchQuery = $state('');
  let actionFilter = $state('');
  let dateFrom = $state('');
  let dateTo = $state('');
  let userIdFilter = $state('');
  // let tenantIdFilter = $state("");

  // Debounced reload (search + filters)
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
    // tenantIdFilter = "";
    page = 1;
    void loadLogs();
  }

  async function loadLogs() {
    if (!$isSuperAdmin) return;

    loading = true;
    try {
      // Prepare filters - remove empty strings
      const activeFilters: any = {};
      if (searchQuery) activeFilters.search = searchQuery;
      if (actionFilter) activeFilters.action = actionFilter;
      if (dateFrom) activeFilters.date_from = new Date(dateFrom).toISOString();
      if (dateTo) activeFilters.date_to = new Date(dateTo).toISOString();
      if (userIdFilter) activeFilters.user_id = userIdFilter;
      // if (tenantIdFilter) activeFilters.tenant_id = tenantIdFilter;

      const res = await api.superadmin.listAuditLogs(page, pageSize, activeFilters);
      logs = res.data;
      total = res.total;
    } catch (err) {
      console.error('Failed to load audit logs:', err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    let cleanup: (() => void) | undefined;

    const unsubscribe = isSuperAdmin.subscribe((value) => {
      if (!value) {
        goto('/dashboard');
        return;
      }
    });

    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(max-width: 899px)'); // Match global.css
      const sync = () => (isMobile = mq.matches);
      sync();
      try {
        mq.addEventListener('change', sync);
        cleanup = () => {
          mq.removeEventListener('change', sync);
          unsubscribe();
        };
      } catch {
        // @ts-ignore
        mq.addListener?.(sync);
        // @ts-ignore
        cleanup = () => {
          mq.removeListener?.(sync);
          unsubscribe();
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

<div class="sa-audit fade-in">
  <div class="page-head">
    <div>
      <div class="crumbs">
        {$t('superadmin.audit_logs.crumbs.root')}
        <span class="crumb-sep">›</span>
        <b>{$t('superadmin.audit_logs.crumbs.audit_logs')}</b>
      </div>
      <h1>{$t('superadmin.audit_logs.title') || 'Audit Logs'}</h1>
    </div>
  </div>

  <div class="panel">
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
  </div>
</div>

<style>
  .sa-audit {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1280px;
    margin: 0 auto;
    color: var(--text-primary);
  }

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

  .panel {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    overflow: hidden;
  }

  @media (max-width: 768px) {
    .page-head { align-items: flex-start; flex-direction: column; }
  }
</style>
