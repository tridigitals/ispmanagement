<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import { isSuperAdmin } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
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

<div class="superadmin-content fade-in">
  <div class="glass-card">
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
  .superadmin-content {
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
</style>
