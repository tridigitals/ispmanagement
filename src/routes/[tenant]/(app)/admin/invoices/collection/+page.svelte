<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';
  import { api, type BillingCollectionLogView, type BillingCollectionRunResult, type InvoiceReminderLogView } from '$lib/api/client';
  import { appSettings } from '$lib/stores/settings';
  import { can, user, tenant } from '$lib/stores/auth';
  import { toast } from '$lib/stores/toast';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Table from '$lib/components/ui/Table.svelte';
  import { formatDateTime, timeAgo } from '$lib/utils/date';
  import { exportCsvRows, exportExcelRows } from '$lib/utils/tabularExport';
  import { resolveTenantContext } from '$lib/utils/tenantRouting';

  type ActiveTab = 'collection' | 'reminders';

  let activeTab = $state<ActiveTab>('collection');
  let loadingCollection = $state(true);
  let loadingReminders = $state(true);
  let runningNow = $state(false);
  let exportMenuOpen = $state(false);
  let ready = $state(false);

  let collectionRows = $state<BillingCollectionLogView[]>([]);
  let reminderRows = $state<InvoiceReminderLogView[]>([]);
  let lastRunResult = $state<BillingCollectionRunResult | null>(null);

  let collectionAction = $state('all');
  let collectionResult = $state('all');
  let collectionSearch = $state('');
  let collectionFrom = $state('');
  let collectionTo = $state('');
  let collectionLimit = $state(200);

  let reminderCode = $state('all');
  let reminderStatus = $state('all');
  let reminderSearch = $state('');
  let reminderFrom = $state('');
  let reminderTo = $state('');
  let reminderLimit = $state(200);

  const tenantCtx = $derived.by(() =>
    resolveTenantContext({
      hostname: $page.url.hostname,
      userTenantSlug: $user?.tenant_slug,
      tenantSlug: $tenant?.slug,
      routeTenantSlug: $page.params.tenant,
    }),
  );
  const tenantPrefix = $derived(tenantCtx.tenantPrefix);
  const invoicesPath = $derived(`${tenantPrefix}/admin/invoices`);

  const collectionColumns = $derived.by(() => [
    { key: 'created_at', label: $t('admin.billing_collection.columns.time') || 'Time', width: '180px' },
    { key: 'invoice', label: $t('admin.billing_collection.columns.invoice') || 'Invoice', width: '160px' },
    { key: 'customer', label: $t('admin.billing_collection.columns.customer') || 'Customer', width: '180px' },
    { key: 'action', label: $t('admin.billing_collection.columns.action') || 'Action', width: '120px' },
    { key: 'result', label: $t('admin.billing_collection.columns.result') || 'Result', width: '110px' },
    { key: 'subscription_status', label: $t('admin.billing_collection.columns.subscription_status') || 'Subscription', width: '140px' },
    { key: 'reason', label: $t('admin.billing_collection.columns.reason') || 'Reason' },
  ]);

  const reminderColumns = $derived.by(() => [
    { key: 'created_at', label: $t('admin.billing_collection.columns.time') || 'Time', width: '180px' },
    { key: 'invoice', label: $t('admin.billing_collection.columns.invoice') || 'Invoice', width: '160px' },
    { key: 'reminder_code', label: $t('admin.billing_collection.columns.reminder_code') || 'Reminder', width: '120px' },
    { key: 'channel', label: $t('admin.billing_collection.columns.channel') || 'Channel', width: '120px' },
    { key: 'recipient', label: $t('admin.billing_collection.columns.recipient') || 'Recipient', width: '220px' },
    { key: 'status', label: $t('admin.billing_collection.columns.status') || 'Status', width: '120px' },
    { key: 'detail', label: $t('admin.billing_collection.columns.detail') || 'Detail' },
  ]);

  const collectionActionOptions = $derived.by(() => {
    const fromData = collectionRows.map((row) => row.action).filter(Boolean);
    return Array.from(new Set(['reminder', 'suspend', 'resume', ...fromData]));
  });

  const collectionResultOptions = $derived.by(() => {
    const fromData = collectionRows.map((row) => row.result).filter(Boolean);
    return Array.from(new Set(['success', 'skipped', 'failed', ...fromData]));
  });

  const reminderCodeOptions = $derived.by(() => {
    const fromData = reminderRows.map((row) => row.reminder_code).filter(Boolean);
    return Array.from(new Set(['d7', 'd3', 'd0', ...fromData]));
  });

  const reminderStatusOptions = $derived.by(() => {
    const fromData = reminderRows.map((row) => row.status).filter(Boolean);
    return Array.from(new Set(['sent', 'queued', 'failed', ...fromData]));
  });

  const activeCount = $derived(activeTab === 'collection' ? collectionRows.length : reminderRows.length);
  const currentLoading = $derived(activeTab === 'collection' ? loadingCollection : loadingReminders);

  onMount(async () => {
    if (!$can('read', 'billing') && !$can('manage', 'billing')) {
      goto('/unauthorized');
      return;
    }
    await Promise.all([loadCollection(), loadReminders()]);
    ready = true;
  });

  $effect(() => {
    if (!ready || activeTab !== 'collection') return;
    const _action = collectionAction;
    const _result = collectionResult;
    const _search = collectionSearch;
    const _from = collectionFrom;
    const _to = collectionTo;
    const _limit = collectionLimit;
    const timer = setTimeout(() => void loadCollection(), 280);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (!ready || activeTab !== 'reminders') return;
    const _code = reminderCode;
    const _status = reminderStatus;
    const _search = reminderSearch;
    const _from = reminderFrom;
    const _to = reminderTo;
    const _limit = reminderLimit;
    const timer = setTimeout(() => void loadReminders(), 280);
    return () => clearTimeout(timer);
  });

  function toIsoUtc(input: string): string | undefined {
    const value = input.trim();
    if (!value) return undefined;
    const parsed = new Date(value);
    if (Number.isNaN(parsed.getTime())) return undefined;
    return parsed.toISOString();
  }

  async function loadCollection() {
    loadingCollection = true;
    try {
      collectionRows = await api.payment.listBillingCollectionLogs({
        action: collectionAction === 'all' ? undefined : collectionAction,
        result: collectionResult === 'all' ? undefined : collectionResult,
        search: collectionSearch.trim() || undefined,
        from: toIsoUtc(collectionFrom),
        to: toIsoUtc(collectionTo),
        limit: collectionLimit,
      });
    } catch (e: any) {
      toast.error(
        e?.message || get(t)('admin.billing_collection.toasts.load_collection_failed') || 'Failed to load billing collection logs',
      );
    } finally {
      loadingCollection = false;
    }
  }

  async function loadReminders() {
    loadingReminders = true;
    try {
      reminderRows = await api.payment.listInvoiceReminderLogs({
        reminderCode: reminderCode === 'all' ? undefined : reminderCode,
        status: reminderStatus === 'all' ? undefined : reminderStatus,
        search: reminderSearch.trim() || undefined,
        from: toIsoUtc(reminderFrom),
        to: toIsoUtc(reminderTo),
        limit: reminderLimit,
      });
    } catch (e: any) {
      toast.error(
        e?.message || get(t)('admin.billing_collection.toasts.load_reminders_failed') || 'Failed to load reminder logs',
      );
    } finally {
      loadingReminders = false;
    }
  }

  async function refreshCurrent() {
    if (activeTab === 'collection') await loadCollection();
    else await loadReminders();
  }

  async function runCollectionNow() {
    if (runningNow) return;
    runningNow = true;
    try {
      lastRunResult = await api.payment.runBillingCollectionNow();
      toast.success(get(t)('admin.billing_collection.toasts.run_ok') || 'Billing collection run completed');
      await Promise.all([loadCollection(), loadReminders()]);
    } catch (e: any) {
      toast.error(e?.message || get(t)('admin.billing_collection.toasts.run_failed') || 'Failed to run billing collection');
    } finally {
      runningNow = false;
    }
  }

  function switchTab(next: ActiveTab) {
    if (activeTab === next) return;
    activeTab = next;
    exportMenuOpen = false;
  }

  function clearFilters() {
    if (activeTab === 'collection') {
      collectionAction = 'all';
      collectionResult = 'all';
      collectionSearch = '';
      collectionFrom = '';
      collectionTo = '';
      collectionLimit = 200;
      void loadCollection();
      return;
    }

    reminderCode = 'all';
    reminderStatus = 'all';
    reminderSearch = '';
    reminderFrom = '';
    reminderTo = '';
    reminderLimit = 200;
    void loadReminders();
  }

  function resultTone(result: string) {
    const x = String(result || '').toLowerCase();
    if (x === 'success' || x === 'sent') return 'tone-success';
    if (x === 'failed') return 'tone-danger';
    return 'tone-warn';
  }

  function normalizeAction(value: string) {
    const x = String(value || '').toLowerCase();
    if (x === 'reminder') return get(t)('admin.billing_collection.actions.reminder') || 'Reminder';
    if (x === 'suspend') return get(t)('admin.billing_collection.actions.suspend') || 'Suspend';
    if (x === 'resume') return get(t)('admin.billing_collection.actions.resume') || 'Resume';
    return value || '—';
  }

  function normalizeStatus(value: string) {
    const x = String(value || '').toLowerCase();
    if (x === 'success' || x === 'sent') return get(t)('admin.billing_collection.results.success') || 'Success';
    if (x === 'failed') return get(t)('admin.billing_collection.results.failed') || 'Failed';
    if (x === 'skipped' || x === 'queued') return get(t)('admin.billing_collection.results.skipped') || 'Skipped';
    return value || '—';
  }

  function normalizeReminder(code: string) {
    const x = String(code || '').toLowerCase();
    if (x === 'd7') return get(t)('admin.billing_collection.reminders.d7') || 'H-7';
    if (x === 'd3') return get(t)('admin.billing_collection.reminders.d3') || 'H-3';
    if (x === 'd0') return get(t)('admin.billing_collection.reminders.d0') || 'H-0';
    return code || '—';
  }

  function buildCollectionExportRows() {
    return collectionRows.map((row) => ({
      time: formatDateTime(row.created_at, { timeZone: $appSettings.app_timezone }),
      invoice: row.invoice_number || row.invoice_id,
      customer: row.customer_name || '—',
      action: row.action,
      result: row.result,
      invoice_status: row.invoice_status || '—',
      subscription_status: row.subscription_status || '—',
      reason: row.reason || '',
    }));
  }

  function buildReminderExportRows() {
    return reminderRows.map((row) => ({
      time: formatDateTime(row.created_at, { timeZone: $appSettings.app_timezone }),
      invoice: row.invoice_number || row.invoice_id,
      reminder_code: row.reminder_code,
      channel: row.channel || '—',
      recipient: row.recipient || '—',
      status: row.status,
      detail: row.detail || '',
    }));
  }

  function exportCsv() {
    const rows = activeTab === 'collection' ? buildCollectionExportRows() : buildReminderExportRows();
    const ok = exportCsvRows(rows, activeTab === 'collection' ? 'billing-collection-logs' : 'invoice-reminder-logs');
    if (!ok) {
      toast.error(get(t)('admin.billing_collection.toasts.no_data_export') || 'No data to export');
      return;
    }
    toast.success(get(t)('admin.billing_collection.toasts.export_ok') || 'Export completed');
  }

  function exportExcel() {
    const rows = activeTab === 'collection' ? buildCollectionExportRows() : buildReminderExportRows();
    const ok = exportExcelRows(rows, activeTab === 'collection' ? 'billing-collection-logs' : 'invoice-reminder-logs');
    if (!ok) {
      toast.error(get(t)('admin.billing_collection.toasts.no_data_export') || 'No data to export');
      return;
    }
    toast.success(get(t)('admin.billing_collection.toasts.export_ok') || 'Export completed');
  }

  function toggleExportMenu() {
    exportMenuOpen = !exportMenuOpen;
  }
</script>

<div class="page-container fade-in">
  <nav class="breadcrumb" aria-label="Breadcrumb">
    <button class="crumb-link" type="button" onclick={() => goto(invoicesPath)}>
      {$t('sidebar.invoices') || 'Invoices'}
    </button>
    <span class="crumb-sep">/</span>
    <span class="crumb-current">{$t('admin.billing_collection.title') || 'Billing Collection Logs'}</span>
  </nav>

  <div class="page-header">
    <div>
      <h1>{$t('admin.billing_collection.title') || 'Billing Collection Logs'}</h1>
      <p class="subtitle">
        {$t('admin.billing_collection.subtitle') ||
          'Monitor reminder delivery, suspend/resume actions, and run collection manually.'}
      </p>
    </div>
    <div class="header-actions">
      <button class="btn btn-secondary" type="button" onclick={refreshCurrent}>
        <Icon name="refresh-cw" size={16} />
        {$t('common.refresh') || 'Refresh'}
      </button>
      <button class="btn btn-primary" type="button" onclick={runCollectionNow} disabled={runningNow}>
        <Icon name="play" size={16} />
        {runningNow
          ? $t('admin.billing_collection.actions.running') || 'Running...'
          : $t('admin.billing_collection.actions.run_now') || 'Run Now'}
      </button>
      <div class="export-wrap">
        <button class="btn btn-secondary" type="button" onclick={toggleExportMenu}>
          <Icon name="download" size={16} />
          {$t('admin.billing_collection.actions.export') || 'Export'}
          <Icon name="chevron-down" size={14} />
        </button>
        {#if exportMenuOpen}
          <button
            class="export-backdrop"
            type="button"
            onclick={() => {
              exportMenuOpen = false;
            }}
            aria-label={$t('common.close') || 'Close'}
          ></button>
          <div class="export-menu">
            <button
              class="export-item"
              type="button"
              onclick={() => {
                exportMenuOpen = false;
                exportCsv();
              }}
            >
              {$t('admin.billing_collection.export.csv') || 'Export CSV'}
            </button>
            <button
              class="export-item"
              type="button"
              onclick={() => {
                exportMenuOpen = false;
                exportExcel();
              }}
            >
              {$t('admin.billing_collection.export.excel') || 'Export Excel'}
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>

  {#if lastRunResult}
    <div class="run-summary">
      <div class="summary-item">
        <span>{$t('admin.billing_collection.summary.evaluated') || 'Evaluated'}</span>
        <strong>{lastRunResult.evaluated_count}</strong>
      </div>
      <div class="summary-item">
        <span>{$t('admin.billing_collection.summary.reminders_sent') || 'Reminders sent'}</span>
        <strong>{lastRunResult.reminder_sent_count}</strong>
      </div>
      <div class="summary-item">
        <span>{$t('admin.billing_collection.summary.suspended') || 'Suspended'}</span>
        <strong>{lastRunResult.suspended_count}</strong>
      </div>
      <div class="summary-item">
        <span>{$t('admin.billing_collection.summary.resumed') || 'Resumed'}</span>
        <strong>{lastRunResult.resumed_count}</strong>
      </div>
      <div class="summary-item">
        <span>{$t('admin.billing_collection.summary.failed') || 'Failed'}</span>
        <strong>{lastRunResult.failed_count}</strong>
      </div>
    </div>
  {/if}

  <div class="card content-card">
    <div class="tabs">
      <button class="tab-btn" class:active={activeTab === 'collection'} type="button" onclick={() => switchTab('collection')}>
        {$t('admin.billing_collection.tabs.collection') || 'Collection Logs'}
        <span>{collectionRows.length}</span>
      </button>
      <button class="tab-btn" class:active={activeTab === 'reminders'} type="button" onclick={() => switchTab('reminders')}>
        {$t('admin.billing_collection.tabs.reminders') || 'Reminder Logs'}
        <span>{reminderRows.length}</span>
      </button>
    </div>

    {#if activeTab === 'collection'}
      <div class="filter-row">
        <select class="select-input" bind:value={collectionAction}>
          <option value="all">{$t('admin.billing_collection.filters.all_actions') || 'All actions'}</option>
          {#each collectionActionOptions as option}
            <option value={option}>{normalizeAction(option)}</option>
          {/each}
        </select>
        <select class="select-input" bind:value={collectionResult}>
          <option value="all">{$t('admin.billing_collection.filters.all_results') || 'All results'}</option>
          {#each collectionResultOptions as option}
            <option value={option}>{normalizeStatus(option)}</option>
          {/each}
        </select>
        <input
          class="text-input"
          type="text"
          bind:value={collectionSearch}
          placeholder={$t('admin.billing_collection.filters.search_placeholder') || 'Search invoice/customer/reason...'}
        />
        <input class="text-input" type="datetime-local" bind:value={collectionFrom} title={$t('common.from') || 'From'} />
        <input class="text-input" type="datetime-local" bind:value={collectionTo} title={$t('common.to') || 'To'} />
        <select class="select-input limit-input" bind:value={collectionLimit}>
          <option value={100}>100</option>
          <option value={200}>200</option>
          <option value={500}>500</option>
          <option value={1000}>1000</option>
        </select>
        <button class="btn btn-clear" type="button" onclick={clearFilters}>
          {$t('admin.billing_collection.filters.clear') || 'Clear'}
        </button>
      </div>
    {:else}
      <div class="filter-row">
        <select class="select-input" bind:value={reminderCode}>
          <option value="all">{$t('admin.billing_collection.filters.all_reminders') || 'All reminders'}</option>
          {#each reminderCodeOptions as option}
            <option value={option}>{normalizeReminder(option)}</option>
          {/each}
        </select>
        <select class="select-input" bind:value={reminderStatus}>
          <option value="all">{$t('admin.billing_collection.filters.all_statuses') || 'All statuses'}</option>
          {#each reminderStatusOptions as option}
            <option value={option}>{normalizeStatus(option)}</option>
          {/each}
        </select>
        <input
          class="text-input"
          type="text"
          bind:value={reminderSearch}
          placeholder={$t('admin.billing_collection.filters.search_reminder') || 'Search invoice/detail...'}
        />
        <input class="text-input" type="datetime-local" bind:value={reminderFrom} title={$t('common.from') || 'From'} />
        <input class="text-input" type="datetime-local" bind:value={reminderTo} title={$t('common.to') || 'To'} />
        <select class="select-input limit-input" bind:value={reminderLimit}>
          <option value={100}>100</option>
          <option value={200}>200</option>
          <option value={500}>500</option>
          <option value={1000}>1000</option>
        </select>
        <button class="btn btn-clear" type="button" onclick={clearFilters}>
          {$t('admin.billing_collection.filters.clear') || 'Clear'}
        </button>
      </div>
    {/if}

    <div class="result-meta">
      <span>{$t('admin.billing_collection.meta.showing') || 'Showing'} {activeCount} {$t('common.results') || 'results'}</span>
    </div>

    {#if activeTab === 'collection'}
      <Table columns={collectionColumns} data={collectionRows} loading={currentLoading}>
        {#snippet cell({ item, column })}
          {#if column.key === 'created_at'}
            <div class="time-cell">
              <strong>{formatDateTime(item.created_at, { timeZone: $appSettings.app_timezone })}</strong>
              <small>{timeAgo(item.created_at)}</small>
            </div>
          {:else if column.key === 'invoice'}
            <div>
              <strong>{item.invoice_number || item.invoice_id}</strong>
              <small class="muted">{item.invoice_status || '—'}</small>
            </div>
          {:else if column.key === 'customer'}
            <div>
              <strong>{item.customer_name || '—'}</strong>
              <small class="muted">{item.due_date ? formatDateTime(item.due_date, { timeZone: $appSettings.app_timezone }) : '—'}</small>
            </div>
          {:else if column.key === 'action'}
            <span class="pill">{normalizeAction(item.action)}</span>
          {:else if column.key === 'result'}
            <span class={`pill ${resultTone(item.result)}`}>{normalizeStatus(item.result)}</span>
          {:else if column.key === 'subscription_status'}
            <span class="muted">{item.subscription_status || '—'}</span>
          {:else if column.key === 'reason'}
            <span class="reason">{item.reason || '—'}</span>
          {/if}
        {/snippet}
      </Table>
    {:else}
      <Table columns={reminderColumns} data={reminderRows} loading={currentLoading}>
        {#snippet cell({ item, column })}
          {#if column.key === 'created_at'}
            <div class="time-cell">
              <strong>{formatDateTime(item.created_at, { timeZone: $appSettings.app_timezone })}</strong>
              <small>{timeAgo(item.created_at)}</small>
            </div>
          {:else if column.key === 'invoice'}
            <div>
              <strong>{item.invoice_number || item.invoice_id}</strong>
              <small class="muted">{item.due_date ? formatDateTime(item.due_date, { timeZone: $appSettings.app_timezone }) : '—'}</small>
            </div>
          {:else if column.key === 'reminder_code'}
            <span class="pill">{normalizeReminder(item.reminder_code)}</span>
          {:else if column.key === 'channel'}
            <span class="muted">{item.channel || '—'}</span>
          {:else if column.key === 'recipient'}
            <span class="reason">{item.recipient || '—'}</span>
          {:else if column.key === 'status'}
            <span class={`pill ${resultTone(item.status)}`}>{normalizeStatus(item.status)}</span>
          {:else if column.key === 'detail'}
            <span class="reason">{item.detail || '—'}</span>
          {/if}
        {/snippet}
      </Table>
    {/if}
  </div>
</div>

<style>
  .page-container {
    padding: clamp(1rem, 3vw, 2rem);
    max-width: 1520px;
    margin: 0 auto;
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.75rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    flex-wrap: wrap;
  }

  .crumb-link {
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    padding: 0;
    cursor: pointer;
  }

  .crumb-link:hover {
    color: var(--text-primary);
    text-decoration: underline;
  }

  .crumb-current {
    color: var(--text-primary);
    font-weight: 600;
  }

  .crumb-sep {
    color: var(--text-tertiary);
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }

  h1 {
    margin: 0;
    font-size: 1.7rem;
    font-weight: 700;
  }

  .subtitle {
    margin: 0.35rem 0 0;
    color: var(--text-secondary);
  }

  .header-actions {
    display: flex;
    gap: 0.65rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .run-summary {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .summary-item {
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .summary-item span {
    color: var(--text-secondary);
    font-size: 0.82rem;
  }

  .summary-item strong {
    font-size: 1.12rem;
    color: var(--text-primary);
  }

  .content-card {
    border: 1px solid var(--border-color);
    border-radius: 12px;
    background: var(--bg-surface);
    overflow: hidden;
  }

  .tabs {
    display: flex;
    gap: 0.5rem;
    padding: 0.9rem;
    border-bottom: 1px solid var(--border-color);
  }

  .tab-btn {
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-secondary);
    border-radius: 9px;
    padding: 0.55rem 0.9rem;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .tab-btn span {
    font-size: 0.78rem;
    color: var(--text-tertiary);
  }

  .tab-btn.active {
    background: color-mix(in srgb, var(--color-primary) 14%, var(--bg-surface) 86%);
    color: var(--text-primary);
    border-color: color-mix(in srgb, var(--color-primary) 48%, var(--border-color) 52%);
  }

  .filter-row {
    padding: 0.9rem;
    border-bottom: 1px solid var(--border-color);
    display: grid;
    grid-template-columns: minmax(160px, 0.9fr) minmax(160px, 0.9fr) minmax(280px, 1.5fr) minmax(170px, 1fr) minmax(170px, 1fr) 120px auto;
    gap: 0.6rem;
  }

  .select-input,
  .text-input {
    min-height: 38px;
    border: 1px solid var(--border-color);
    background: var(--bg-surface);
    color: var(--text-primary);
    border-radius: 8px;
    padding: 0 0.7rem;
  }

  .limit-input {
    max-width: 110px;
  }

  .result-meta {
    padding: 0.7rem 0.9rem;
    color: var(--text-secondary);
    font-size: 0.87rem;
    border-bottom: 1px solid var(--border-color);
  }

  .time-cell {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .time-cell strong {
    font-size: 0.86rem;
  }

  .time-cell small,
  .muted {
    color: var(--text-secondary);
    font-size: 0.8rem;
  }

  .reason {
    color: var(--text-primary);
    word-break: break-word;
  }

  .pill {
    border: 1px solid var(--border-color);
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
    font-size: 0.76rem;
    font-weight: 700;
    color: var(--text-primary);
    display: inline-flex;
    align-items: center;
  }

  .tone-success {
    color: var(--color-success, #10b981);
    border-color: color-mix(in srgb, var(--color-success, #10b981) 40%, var(--border-color) 60%);
    background: color-mix(in srgb, var(--color-success, #10b981) 12%, transparent 88%);
  }

  .tone-danger {
    color: var(--color-danger, #ef4444);
    border-color: color-mix(in srgb, var(--color-danger, #ef4444) 40%, var(--border-color) 60%);
    background: color-mix(in srgb, var(--color-danger, #ef4444) 12%, transparent 88%);
  }

  .tone-warn {
    color: var(--color-warning, #f59e0b);
    border-color: color-mix(in srgb, var(--color-warning, #f59e0b) 40%, var(--border-color) 60%);
    background: color-mix(in srgb, var(--color-warning, #f59e0b) 12%, transparent 88%);
  }

  .btn {
    border: 1px solid var(--border-color);
    border-radius: 9px;
    padding: 0.52rem 0.9rem;
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-primary {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: #fff;
  }

  .btn-secondary,
  .btn-clear {
    background: var(--bg-surface);
    color: var(--text-primary);
  }

  .export-wrap {
    position: relative;
  }

  .export-backdrop {
    position: fixed;
    inset: 0;
    background: transparent;
    border: 0;
    z-index: 9;
  }

  .export-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 0.35rem);
    width: 170px;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--bg-surface);
    box-shadow: var(--shadow-md);
    z-index: 10;
    padding: 0.35rem;
  }

  .export-item {
    width: 100%;
    text-align: left;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    padding: 0.5rem 0.65rem;
    border-radius: 7px;
    cursor: pointer;
  }

  .export-item:hover {
    background: var(--bg-hover);
  }

  @media (max-width: 1080px) {
    .filter-row {
      grid-template-columns: 1fr 1fr 1fr;
    }

    .limit-input {
      max-width: none;
    }
  }
  @media (max-width: 760px) {
    .filter-row {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 720px) {
    .page-header {
      flex-direction: column;
      align-items: stretch;
    }

    .header-actions {
      width: 100%;
    }

    .header-actions .btn {
      flex: 1;
      justify-content: center;
    }

    .tabs {
      flex-direction: column;
    }
  }
</style>
