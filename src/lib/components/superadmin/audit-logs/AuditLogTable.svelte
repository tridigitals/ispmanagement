<script lang="ts">
  import Table from '$lib/components/ui/Table.svelte';
  import Pagination from '$lib/components/ui/Pagination.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import AuditLogDetails from './AuditLogDetails.svelte';
  import type { AuditLog } from '$lib/api/client';
  import { t } from 'svelte-i18n';
  import { appSettings } from '$lib/stores/settings';
  import { formatDateTime } from '$lib/utils/date';

  let { logs, loading, page, total, pageSize, viewMode, isMobile, onPageChange, onPageSizeChange } =
    $props<{
      logs: AuditLog[];
      loading: boolean;
      page: number;
      total: number;
      pageSize: number;
      viewMode: 'table' | 'cards';
      isMobile: boolean;
      onPageChange: (page: number) => void;
      onPageSizeChange: (size: number) => void;
    }>();

  let expandedLogId = $state<string | null>(null);

  function toggleExpand(id: string) {
    expandedLogId = expandedLogId === id ? null : id;
  }

  function getActionCategory(action: string) {
    const head = String(action || '')
      .split('_')[0]
      .toLowerCase();
    if (['auth', 'login', 'logout', '2fa'].includes(head)) return 'auth';
    if (['user', 'users'].includes(head)) return 'user';
    if (['tenant', 'tenants'].includes(head)) return 'tenant';
    if (['setting', 'settings'].includes(head)) return 'settings';
    return 'other';
  }

  // Condensed columns configuration
  const columns = $derived.by(() => [
    {
      key: 'created_at',
      label: $t('superadmin.audit_logs.columns.time_ip') || 'Time / IP',
      width: '185px',
    },
    {
      key: 'user',
      label: $t('superadmin.audit_logs.columns.user_tenant') || 'User / Tenant',
      width: '240px',
    },
    {
      key: 'action',
      label: $t('superadmin.audit_logs.columns.action_resource') || 'Action / Resource',
      width: '220px',
    },
    {
      key: 'details',
      label: $t('superadmin.audit_logs.columns.details') || 'Details',
    },
  ]);
</script>

{#if viewMode === 'cards' || isMobile}
  <div class="cards-wrapper">
    {#if loading && logs.length === 0}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>
          {$t('superadmin.audit_logs.loading') || 'Loading logs...'}
        </p>
      </div>
    {:else if logs.length === 0}
      <div class="empty-state">
        <Icon name="activity" size={48} />
        <h4>
          {$t('superadmin.audit_logs.empty.title') || 'No logs found'}
        </h4>
        <p>
          {$t('superadmin.audit_logs.empty.subtitle') || 'Try adjusting your filters.'}
        </p>
      </div>
    {:else}
      <div class="log-cards" aria-label={$t('superadmin.audit_logs.aria.cards') || 'Audit logs'}>
        {#each logs as l (l.id)}
          <div class="log-card">
            <div class="log-top">
              <div class="log-left">
                <div class="log-time">
                  {formatDateTime(l.created_at, { timeZone: $appSettings.app_timezone })}
                </div>
                <span class="action-pill {getActionCategory(l.action)}">
                  {l.action}
                </span>
              </div>
              <button
                type="button"
                class="btn-icon"
                onclick={() => toggleExpand(l.id)}
                aria-label={expandedLogId === l.id
                  ? $t('superadmin.audit_logs.actions.collapse_details') || 'Collapse details'
                  : $t('superadmin.audit_logs.actions.expand_details') || 'Expand details'}
                title={expandedLogId === l.id
                  ? $t('superadmin.audit_logs.actions.collapse') || 'Collapse'
                  : $t('superadmin.audit_logs.actions.expand') || 'Expand'}
              >
                <Icon name={expandedLogId === l.id ? 'chevron-up' : 'chevron-down'} size={18} />
              </button>
            </div>

            <div class="log-grid">
              <div class="kv">
                <span class="k">{$t('superadmin.audit_logs.labels.user') || 'User'}</span>
                <span class="v">
                  {#if l.user_email}
                    {l.user_name ? `${l.user_name} — ${l.user_email}` : l.user_email}
                  {:else if l.user_id}
                    <span class="text-mono">{l.user_id.substring(0, 8)}…</span>
                  {:else}
                    —
                  {/if}
                </span>
              </div>

              <div class="kv">
                <span class="k">{$t('superadmin.audit_logs.labels.tenant') || 'Tenant'}</span>
                <span class="v">
                  {#if l.tenant_name}
                    {l.tenant_name}
                  {:else if l.tenant_id}
                    <span class="text-mono">{l.tenant_id.substring(0, 8)}…</span>
                  {:else}
                    <span class="badge-global">{$t('common.global') || 'Global'}</span>
                  {/if}
                </span>
              </div>

              <div class="kv">
                <span class="k">{$t('superadmin.audit_logs.labels.resource') || 'Resource'}</span>
                <span class="v">
                  {l.resource}
                  {#if l.resource_name}
                    <span class="sub">{l.resource_name}</span>
                  {:else if l.resource_id}
                    <span class="sub text-mono">{l.resource_id.substring(0, 8)}…</span>
                  {/if}
                </span>
              </div>

              <div class="kv">
                <span class="k">{$t('superadmin.audit_logs.labels.ip') || 'IP'}</span>
                <span class="v text-mono">{l.ip_address || $t('common.na') || '—'}</span>
              </div>
            </div>

            {#if expandedLogId === l.id}
              <AuditLogDetails details={l.details} />
            {/if}
          </div>
        {/each}
      </div>

      <div class="cards-pagination">
        <Pagination
          count={total}
          page={page - 1}
          {pageSize}
          onchange={onPageChange}
          onpageSizeChange={onPageSizeChange}
        />
      </div>
    {/if}
  </div>
{:else}
  <div class="table-wrapper">
    <Table
      {columns}
      data={logs}
      {loading}
      pagination={true}
      {pageSize}
      count={total}
      onchange={onPageChange}
      onpageSizeChange={onPageSizeChange}
      serverSide={true}
      mobileView="card"
    >
      {#snippet cell({ item, key })}
        {#if key === 'created_at'}
          <div class="stack">
            <span class="text-secondary font-medium"
              >{formatDateTime(item.created_at, { timeZone: $appSettings.app_timezone })}</span
            >
            <span class="text-xs text-mono text-muted">{item.ip_address || '—'}</span>
          </div>
        {:else if key === 'user'}
          <div class="stack">
            <div class="user-info">
              <span class="user-name">{item.user_name || 'System'}</span>
              {#if item.user_email}
                <span class="user-email">{item.user_email}</span>
              {/if}
            </div>
            <div class="tenant-tag">
              {#if item.tenant_name}
                <span class="tenant-label">{item.tenant_name}</span>
              {:else}
                <span class="badge-global">{$t('common.global') || 'Global'}</span>
              {/if}
            </div>
          </div>
        {:else if key === 'action'}
          <div class="stack">
            <span class="action-pill {getActionCategory(item.action)}">{item.action}</span>
            <div class="resource-info">
              <span class="text-xs font-medium">{item.resource}</span>
              {#if item.resource_name}
                <span class="text-xs text-muted">({item.resource_name})</span>
              {/if}
            </div>
          </div>
        {:else if key === 'details'}
          <div class="details-cell" title={item.details}>
            {item.details || $t('common.na') || '—'}
          </div>
        {/if}
      {/snippet}
    </Table>
  </div>
{/if}

<style>
  .table-wrapper {
    padding: 0 1.25rem 1rem 1.25rem;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    width: 100%;
    overflow: hidden;
  }

  .cards-wrapper {
    padding: 0 1.25rem 1rem 1.25rem;
  }

  /* Cell Styles */
  .stack {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    line-height: 1.3;
  }

  .user-info {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }

  .tenant-label {
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-weight: 600;
    background: rgba(255, 255, 255, 0.05);
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    display: inline-block;
  }

  :global([data-theme='light']) .tenant-label {
    background: rgba(0, 0, 0, 0.03);
  }

  .resource-info {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .action-pill {
    display: inline-block;
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    width: max-content;
  }

  .action-pill.auth {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
  }
  .action-pill.user {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
  }
  .action-pill.tenant {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
  }
  .action-pill.settings {
    background: rgba(139, 92, 246, 0.15);
    color: #a78bfa;
  }
  .action-pill.other {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-secondary);
  }

  :global([data-theme='light']) .action-pill.auth {
    background: rgba(245, 158, 11, 0.12);
    color: #d97706;
  }
  :global([data-theme='light']) .action-pill.user {
    background: rgba(59, 130, 246, 0.12);
    color: #2563eb;
  }
  :global([data-theme='light']) .action-pill.tenant {
    background: rgba(16, 185, 129, 0.12);
    color: #059669;
  }
  :global([data-theme='light']) .action-pill.settings {
    background: rgba(139, 92, 246, 0.12);
    color: #7c3aed;
  }
  :global([data-theme='light']) .action-pill.other {
    background: rgba(0, 0, 0, 0.06);
    color: #4b5563;
  }

  .details-cell {
    max-width: 400px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 0.85rem;
  }

  .user-name {
    font-weight: 600;
    color: var(--text-primary);
  }

  .user-email {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .badge-global {
    font-size: 0.7rem;
    background: rgba(99, 102, 241, 0.15);
    color: #818cf8;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 700;
  }

  :global([data-theme='light']) .badge-global {
    color: #4f46e5;
    background: rgba(99, 102, 241, 0.1);
  }

  .text-secondary {
    color: var(--text-secondary);
  }

  .text-muted {
    color: var(--text-muted);
  }

  .text-xs {
    font-size: 0.75rem;
  }

  .font-medium {
    font-weight: 500;
  }

  .text-mono {
    font-family: var(--font-mono);
  }

  /* Card view specific */
  .log-cards {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .log-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1rem;
  }

  .log-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  .log-time {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: 0.25rem;
  }

  .btn-icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.02);
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  :global([data-theme='light']) .btn-icon {
    border-color: rgba(0, 0, 0, 0.06);
    background: rgba(0, 0, 0, 0.02);
  }

  .log-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .kv {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .k {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    font-weight: 600;
  }

  .v {
    font-size: 0.9rem;
    color: var(--text-primary);
    word-break: break-all;
  }

  .sub {
    display: block;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .loading-state,
  .empty-state {
    text-align: center;
    padding: 3rem;
    color: var(--text-secondary);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border-color);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
