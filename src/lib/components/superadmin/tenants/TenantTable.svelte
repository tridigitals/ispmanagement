<script lang="ts">
  import Table from '$lib/components/ui/Table.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import CustomDomainStatusBadge from '$lib/components/domain/CustomDomainStatusBadge.svelte';
  import { fly } from 'svelte/transition';
  import { t } from 'svelte-i18n';
  import { appSettings } from '$lib/stores/settings';
  import { formatDate } from '$lib/utils/date';

  let {
    tenants = [],
    loading = false,
    viewMode = 'table',
    isMobile = false,
    columns = [],
    onEdit,
    onDelete,
    onToggleStatus,
  } = $props<{
    tenants: any[];
    loading: boolean;
    viewMode: 'table' | 'cards';
    isMobile: boolean;
    columns: any[];
    onEdit: (t: any) => void;
    onDelete: (id: string) => void;
    onToggleStatus: (t: any) => void;
  }>();
</script>

{#if viewMode === 'cards' || isMobile}
  <div class="tenants-grid" aria-label={$t('superadmin.tenants.aria.cards')}>
    {#each tenants as tenant (tenant.id)}
      <div class="tenant-card" in:fly={{ y: 6, duration: 150 }}>
        <div class="tenant-top">
          <div>
            <div class="tenant-name">{tenant.name}</div>
            <div class="tenant-sub">
              <span class="tenant-slug">{tenant.slug}</span>
              {#if tenant.custom_domain}
                <span class="dot">•</span>
                <span class="tenant-domain mono">
                  {tenant.custom_domain}
                </span>
                <CustomDomainStatusBadge
                  customDomain={tenant.custom_domain}
                  status={tenant.custom_domain_status}
                  failureReason={tenant.custom_domain_failure_reason}
                />
              {/if}
            </div>
          </div>
          <span class="status-badge {tenant.is_active ? 'success' : 'error'}">
            {tenant.is_active
              ? $t('common.active') || 'Active'
              : $t('common.inactive') || 'Inactive'}
          </span>
        </div>

        <div class="tenant-meta">
          <span class="meta-label">
            {$t('superadmin.tenants.meta.created')}
          </span>
          <span class="meta-value">
            {tenant.created_at
              ? formatDate(tenant.created_at, { timeZone: $appSettings.app_timezone })
              : '—'}
          </span>
        </div>

        <div class="tenant-actions">
          <button
            class="btn-icon {tenant.is_active ? 'warn' : 'success'}"
            title={tenant.is_active
              ? $t('superadmin.tenants.actions.deactivate') || 'Deactivate'
              : $t('superadmin.tenants.actions.activate') || 'Activate'}
            type="button"
            onclick={() => onToggleStatus(tenant)}
          >
            <Icon name={tenant.is_active ? 'ban' : 'check-circle'} size={18} />
          </button>
          <button
            class="btn-icon"
            title={$t('common.edit')}
            type="button"
            onclick={() => onEdit(tenant)}
          >
            <Icon name="edit" size={18} />
          </button>
          <button
            class="btn-icon danger"
            title={$t('common.delete')}
            type="button"
            onclick={() => onDelete(tenant.id)}
          >
            <Icon name="trash" size={18} />
          </button>
        </div>
      </div>
    {/each}

    {#if tenants.length === 0}
      <div class="empty-state-container">
        <div class="empty-icon">
          <Icon name="database" size={64} />
        </div>
        <h3>
          {$t('superadmin.tenants.empty.title')}
        </h3>
        <p>
          {$t('superadmin.tenants.empty.hint')}
        </p>
      </div>
    {/if}
  </div>
{:else}
  <div class="table-wrapper">
    <Table
      pagination={true}
      {loading}
      data={tenants}
      {columns}
      emptyText={$t('superadmin.tenants.empty.title')}
      mobileView="scroll"
    >
      {#snippet empty()}
        <div class="empty-state-container">
          <div class="empty-icon">
            <Icon name="database" size={64} />
          </div>
          <h3>
            {$t('superadmin.tenants.empty.title')}
          </h3>
          <p>
            {$t('superadmin.tenants.empty.hint')}
          </p>
        </div>
      {/snippet}

      {#snippet cell({ item, key })}
        {#if key === 'custom_domain'}
          {#if item.custom_domain}
            <div class="domain-cell">
              <code class="domain-badge">{item.custom_domain}</code>
              <CustomDomainStatusBadge
                customDomain={item.custom_domain}
                status={item.custom_domain_status}
                failureReason={item.custom_domain_failure_reason}
              />
            </div>
          {:else}
            <span class="text-muted">-</span>
          {/if}
        {:else if key === 'is_active'}
          <span class="status-badge {item.is_active ? 'success' : 'error'}">
            {item.is_active ? $t('common.active') || 'Active' : $t('common.inactive') || 'Inactive'}
          </span>
        {:else if key === 'created_at'}
          {formatDate(item.created_at, { timeZone: $appSettings.app_timezone })}
        {:else if key === 'actions'}
          <div class="actions">
            <button
              class="btn-icon {item.is_active ? 'warn' : 'success'}"
              title={item.is_active
                ? $t('superadmin.tenants.actions.deactivate') || 'Deactivate'
                : $t('superadmin.tenants.actions.activate') || 'Activate'}
              type="button"
              onclick={() => onToggleStatus(item)}
            >
              <Icon name={item.is_active ? 'ban' : 'check-circle'} size={18} />
            </button>
            <button
              class="btn-icon"
              title={$t('common.edit')}
              type="button"
              onclick={() => onEdit(item)}
            >
              <Icon name="edit" size={18} />
            </button>
            <button
              class="btn-icon danger"
              title={$t('common.delete')}
              type="button"
              onclick={() => onDelete(item.id)}
            >
              <Icon name="trash" size={18} />
            </button>
          </div>
        {:else}
          {item[key]}
        {/if}
      {/snippet}
    </Table>
  </div>
{/if}

<style>
  .tenants-grid {
    padding: 0 1.25rem 1.25rem 1.25rem;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 1rem;
  }

  .tenant-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1rem;
    box-shadow: var(--shadow-sm);
  }

  :global([data-theme='light']) .tenant-card {
    background: var(--bg-surface);
    border-color: var(--border-color);
    box-shadow: var(--shadow-sm);
  }

  .tenant-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .tenant-name {
    font-weight: 900;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    line-height: 1.15;
  }

  .tenant-sub {
    margin-top: 0.35rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    color: var(--text-secondary);
    font-weight: 650;
    font-size: 0.9rem;
  }

  .tenant-slug {
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
  }

  .dot {
    opacity: 0.6;
  }

  .tenant-domain {
    opacity: 0.9;
  }

  .domain-cell {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .tenant-meta {
    margin-top: 0.9rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-color);
    color: var(--text-secondary);
  }

  :global([data-theme='light']) .tenant-meta {
    border-top-color: var(--border-color);
  }

  .meta-label {
    font-size: 0.8rem;
    font-weight: 750;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .meta-value {
    font-weight: 750;
    color: var(--text-primary);
  }

  .tenant-actions {
    margin-top: 0.9rem;
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .status-badge {
    padding: 0.25rem 0.75rem;
    border-radius: var(--radius-lg);
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .status-badge.success {
    background: var(--bg-success);
    color: var(--color-success);
  }

  .status-badge.error {
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    color: var(--color-danger);
  }

  .domain-badge {
    background: var(--bg-app);
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-family: monospace;
    font-size: 0.85rem;
    color: var(--color-primary);
  }

  .text-muted {
    color: var(--text-secondary);
    font-style: italic;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  :global(.btn-icon.danger:hover:not(:disabled)) {
    background: color-mix(in srgb, var(--color-danger) 10%, transparent);
    border-color: color-mix(in srgb, var(--color-danger) 35%, var(--border-color));
    color: var(--color-danger);
  }

  :global(.btn-icon.warn:hover:not(:disabled)) {
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    border-color: color-mix(in srgb, var(--color-warning) 35%, var(--border-color));
    color: var(--color-warning);
  }

  :global(.btn-icon.success:hover:not(:disabled)) {
    background: var(--bg-success);
    border-color: color-mix(in srgb, var(--color-success) 35%, var(--border-color));
    color: var(--color-success);
  }

  .btn-icon {
    width: 36px;
    height: 36px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }

  :global([data-theme='light']) .btn-icon {
    border-color: var(--border-color);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }

  .btn-icon:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .table-wrapper {
    padding: 0 1.25rem 1rem 1.25rem;
  }

  .empty-state-container {
    padding: 2.25rem 1rem;
    text-align: center;
    color: var(--text-secondary);
  }

  .empty-icon {
    opacity: 0.6;
    margin-bottom: 0.75rem;
  }

  .empty-state-container h3 {
    color: var(--text-primary);
    margin: 0.25rem 0 0.35rem 0;
  }

  .mono {
    font-family:
      ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
      monospace;
  }

  @media (max-width: 768px) {
    .table-wrapper {
      padding: 0 1rem 1rem 1rem;
    }

    .tenants-grid {
      padding: 0 1rem 1rem 1rem;
      grid-template-columns: 1fr;
    }
  }
</style>
