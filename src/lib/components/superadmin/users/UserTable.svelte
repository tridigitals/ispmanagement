<script lang="ts">
  import Table from '$lib/components/ui/Table.svelte';
  import Pagination from '$lib/components/ui/Pagination.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';
  import type { User } from '$lib/api/client';
  import { appSettings } from '$lib/stores/settings';
  import { formatDate } from '$lib/utils/date';

  let {
    users = [],
    loading = false,
    isMobile = false,
    viewMode = 'table',
    currentUser,
    onOpenDetails,
    onReset2FA,
    onToggleActive,
    onDelete,
    getTenantName,
    getInitials,
    page = 1,
    pageSize = 25,
    totalCount = 0,
    onPageChange,
    onPageSizeChange,
  } = $props<{
    users: User[];
    loading: boolean;
    isMobile: boolean;
    viewMode: 'table' | 'cards';
    currentUser: User | null;
    onOpenDetails: (u: User) => void;
    onReset2FA: (u: User) => void;
    onToggleActive: (u: User) => void;
    onDelete?: (u: User) => void;
    getTenantName: (u: any) => string;
    getInitials: (name: string) => string;
    page?: number;
    pageSize?: number;
    totalCount?: number;
    onPageChange: (page: number) => void;
    onPageSizeChange: (pageSize: number) => void;
  }>();

  let pagedUsers = $derived(users);

  const columns = $derived.by(() => [
    { key: 'user', label: $t('superadmin.users.columns.user') || 'User' },
    {
      key: 'email',
      label: $t('superadmin.users.columns.email') || 'Email',
    },
    { key: 'role', label: $t('superadmin.users.columns.role') || 'Role' },
    {
      key: 'tenant',
      label: $t('superadmin.users.columns.tenant') || 'Tenant',
    },
    {
      key: 'status',
      label: $t('superadmin.users.columns.status') || 'Status',
    },
    {
      key: 'joined',
      label: $t('superadmin.users.columns.joined') || 'Joined',
    },
    { key: 'actions', label: '', align: 'right' as const },
  ]);
</script>

{#if loading && users.length === 0}
  <!-- Loading state handled typically by Table or parent, but if completely empty and loading -->
{/if}

{#if viewMode === 'cards' || isMobile}
  <div class="cards-wrapper">
    {#if users.length === 0}
      <div class="empty-state-container">
        <div class="empty-icon">
          <Icon name="users" size={64} />
        </div>
        <h3>
          {$t('superadmin.users.empty.title')}
        </h3>
        <p>
          {$t('superadmin.users.empty.hint')}
        </p>
      </div>
    {:else}
      <div class="user-cards" aria-label={$t('superadmin.users.aria.list')}>
        {#each pagedUsers as u (u.id)}
          <div class="user-card">
            <div class="card-top">
              <div class="user-info">
                <div class="avatar">
                  {getInitials(u.name)}
                </div>
                <div class="user-meta">
                  <div class="user-name">{u.name}</div>
                  <div class="user-email text-muted">
                    {u.email}
                  </div>
                </div>
              </div>

              <div class="actions">
                <button
                  class="btn-icon"
                  onclick={() => onOpenDetails(u)}
                  title={$t('superadmin.users.actions.view_details')}
                  aria-label={$t('superadmin.users.actions.view_details')}
                  type="button"
                >
                  <Icon name="eye" size={16} />
                </button>

                {#if (u as any).two_factor_enabled}
                  <button
                    class="btn-icon warning"
                    onclick={() => onReset2FA(u)}
                    title={$t('superadmin.users.actions.reset_2fa')}
                    aria-label={$t('superadmin.users.actions.reset_2fa')}
                    type="button"
                  >
                    <Icon name="shield-off" size={16} />
                  </button>
                {/if}

                <button
                  class="btn-icon {u.is_active ? 'danger' : 'success'}"
                  onclick={() => onToggleActive(u)}
                  title={u.is_active
                    ? $t('superadmin.users.actions.deactivate_user') || 'Deactivate user'
                    : $t('superadmin.users.actions.activate_user') || 'Activate user'}
                  aria-label={u.is_active
                    ? $t('superadmin.users.actions.deactivate_user') || 'Deactivate user'
                    : $t('superadmin.users.actions.activate_user') || 'Activate user'}
                  disabled={u.is_super_admin || u.id === currentUser?.id}
                  type="button"
                >
                  <Icon name={u.is_active ? 'ban' : 'check-circle'} size={16} />
                </button>

                {#if onDelete}
                  <button
                    class="btn-icon danger"
                    onclick={() => onDelete(u)}
                    title={$t('superadmin.users.actions.delete_user') ||
                      'Delete user permanently'}
                    aria-label={$t('superadmin.users.actions.delete_user') ||
                      'Delete user permanently'}
                    disabled={u.is_super_admin || u.id === currentUser?.id}
                    type="button"
                  >
                    <Icon name="trash" size={16} />
                  </button>
                {/if}
              </div>
            </div>

            <div class="card-bottom">
              <div class="meta-grid">
                <div class="meta-item">
                  <span class="meta-label">{$t('superadmin.users.columns.role')}</span>
                  <span class="meta-value">
                    {#if u.is_super_admin}
                      <span class="role-pill superadmin">
                        {$t('sidebar.super_admin')}
                      </span>
                    {:else if (u as any).tenant_role}
                      <span class="role-pill {(u as any).tenant_role.toLowerCase()}"
                        >{(u as any).tenant_role}</span
                      >
                    {:else}
                      <span class="role-pill {u.role}">{u.role}</span>
                    {/if}
                  </span>
                </div>

                <div class="meta-item">
                  <span class="meta-label">{$t('superadmin.users.columns.tenant')}</span
                  >
                  <span class="meta-value">
                    {#if getTenantName(u as any)}
                      {getTenantName(u as any)}
                    {:else if (u as any).tenant_slug}
                      {(u as any).tenant_slug}
                    {:else}
                      -
                    {/if}
                  </span>
                </div>

                <div class="meta-item">
                  <span class="meta-label">{$t('superadmin.users.columns.status')}</span
                  >
                  <span class="meta-value">
                    {#if u.is_active}
                      <span class="status-pill active">
                        <span class="dot"></span>
                        {$t('common.active')}
                      </span>
                    {:else}
                      <span class="status-pill inactive">
                        <span class="dot"></span>
                        {$t('common.inactive')}
                      </span>
                    {/if}
                  </span>
                </div>

                <div class="meta-item">
                  <span class="meta-label">{$t('superadmin.users.columns.joined')}</span
                  >
                  <span class="meta-value text-muted">
                    {formatDate(u.created_at, { timeZone: $appSettings.app_timezone })}
                  </span>
                </div>
              </div>
            </div>
          </div>
        {/each}
      </div>

      <div class="cards-pagination">
        <Pagination
          count={totalCount}
          page={page - 1}
          pageSize={pageSize}
          onchange={(p: number) => onPageChange(p + 1)}
          onpageSizeChange={onPageSizeChange}
        />
      </div>
    {/if}
  </div>
{:else}
  <div class="table-wrapper">
    <Table
      pagination={true}
      serverSide={true}
      count={totalCount}
      pageSize={pageSize}
      onchange={(p: number) => onPageChange(p + 1)}
      onpageSizeChange={onPageSizeChange}
      {columns}
      data={users}
      {loading}
      emptyText={$t('superadmin.users.empty.title')}
    >
      {#snippet empty()}
        <div class="empty-state-container">
          <div class="empty-icon">
            <Icon name="users" size={64} />
          </div>
          <h3>
            {$t('superadmin.users.empty.title')}
          </h3>
          <p>
            {$t('superadmin.users.empty.subtitle')}
          </p>
        </div>
      {/snippet}

      {#snippet cell({ item, key })}
        {#if key === 'user'}
          <div class="user-info">
            <div class="avatar">
              {getInitials(item.name)}
            </div>
            <div>
              <div class="user-name">{item.name}</div>
            </div>
          </div>
        {:else if key === 'email'}
          {item.email}
        {:else if key === 'role'}
          {#if item.is_super_admin}
            <span class="role-pill superadmin">
              {$t('sidebar.super_admin')}
            </span>
          {:else if item.tenant_role}
            <span class="role-pill {item.tenant_role.toLowerCase()}">{item.tenant_role}</span>
            {#if item.role !== 'user' && item.role !== 'admin' && item.role !== item.tenant_role.toLowerCase()}
              <span class="text-xs text-muted">({item.role})</span>
            {/if}
          {:else}
            <span class="role-pill {item.role}">{item.role}</span>
          {/if}
        {:else if key === 'tenant'}
          {#if getTenantName(item)}
            <div class="tenant-cell">
              <div class="tenant-name">
                {getTenantName(item)}
              </div>
              {#if item.tenant_slug}
                <div class="tenant-meta text-mono">
                  {item.tenant_slug}
                </div>
              {/if}
            </div>
          {:else if item.tenant_slug}
            <div class="tenant-cell">
              <div class="tenant-name">
                {item.tenant_slug}
              </div>
            </div>
          {:else}
            <span class="text-muted">-</span>
          {/if}
        {:else if key === 'status'}
          {#if item.is_active}
            <span class="status-pill active">
              <span class="dot"></span>
              {$t('common.active')}
            </span>
          {:else}
            <span class="status-pill inactive">
              <span class="dot"></span>
              {$t('common.inactive')}
            </span>
          {/if}
        {:else if key === 'joined'}
          <span class="text-muted">
            {formatDate(item.created_at, { timeZone: $appSettings.app_timezone })}
          </span>
        {:else if key === 'actions'}
          <div class="actions">
            <button
              class="btn-icon"
              onclick={() => onOpenDetails(item)}
              title={$t('superadmin.users.actions.view_details')}
              aria-label={$t('superadmin.users.actions.view_details')}
              type="button"
            >
              <Icon name="eye" size={16} />
            </button>

            {#if item.two_factor_enabled}
              <button
                class="btn-icon warning"
                onclick={() => onReset2FA(item)}
                title={$t('superadmin.users.actions.reset_2fa')}
                aria-label={$t('superadmin.users.actions.reset_2fa')}
                type="button"
              >
                <Icon name="shield-off" size={16} />
              </button>
            {/if}

            <button
              class="btn-icon {item.is_active ? 'danger' : 'success'}"
              onclick={() => onToggleActive(item)}
              title={item.is_active
                ? $t('superadmin.users.actions.deactivate_user') || 'Deactivate user'
                : $t('superadmin.users.actions.activate_user') || 'Activate user'}
              aria-label={item.is_active
                ? $t('superadmin.users.actions.deactivate_user') || 'Deactivate user'
                : $t('superadmin.users.actions.activate_user') || 'Activate user'}
              disabled={item.is_super_admin || item.id === currentUser?.id}
              type="button"
            >
              <Icon name={item.is_active ? 'ban' : 'check-circle'} size={16} />
            </button>

            {#if onDelete}
              <button
                class="btn-icon danger"
                onclick={() => onDelete(item)}
                title={$t('superadmin.users.actions.delete_user') ||
                  'Delete user permanently'}
                aria-label={$t('superadmin.users.actions.delete_user') ||
                  'Delete user permanently'}
                disabled={item.is_super_admin || item.id === currentUser?.id}
                type="button"
              >
                <Icon name="trash" size={16} />
              </button>
            {/if}
          </div>
        {:else}
          {item[key]}
        {/if}
      {/snippet}
    </Table>
  </div>
{/if}

<style>
  .table-wrapper {
    padding: 0 1.25rem 1rem 1.25rem;
  }

  .user-info {
    display: flex;
    align-items: center;
    gap: 0.85rem;
  }

  .avatar {
    width: 42px;
    height: 42px;
    background: var(--bg-surface);
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    color: white;
    font-size: 0.9rem;
    flex-shrink: 0;
  }

  .user-name {
    font-weight: 600;
    color: var(--text-primary);
  }

  .text-xs {
    font-size: 0.78rem;
  }

  .tenant-cell {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
  }

  .tenant-name {
    font-weight: 650;
    color: var(--text-primary);
    line-height: 1.15;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tenant-meta {
    opacity: 0.85;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .role-pill {
    padding: 0.3rem 0.8rem;
    border-radius: var(--radius-lg);
    font-size: 0.8rem;
    font-weight: 600;
    text-transform: capitalize;
  }

  .role-pill.admin {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
  }

  .role-pill.superadmin {
    background: var(--color-primary-subtle);
    color: var(--color-primary);
    border: 1px solid color-mix(in srgb, var(--color-primary) 30%, var(--border-color));
  }

  .role-pill.user {
    background: var(--bg-success);
    color: var(--text-success);
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.8rem;
    border-radius: var(--radius-lg);
    font-size: 0.8rem;
    font-weight: 600;
  }

  .status-pill.active {
    background: var(--bg-success);
    color: var(--color-success);
    border: 1px solid color-mix(in srgb, var(--color-success) 24%, var(--border-color));
  }

  .status-pill.inactive {
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    color: var(--color-danger);
    border: 1px solid color-mix(in srgb, var(--color-danger) 24%, var(--border-color));
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  .text-muted {
    color: var(--text-muted);
  }
  .text-mono {
    font-family: monospace;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .btn-icon:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
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

  .btn-icon.warning:hover {
    background: color-mix(in srgb, var(--color-warning) 14%, transparent);
    color: var(--color-warning);
  }

  .btn-icon.danger:hover {
    background: color-mix(in srgb, var(--color-danger) 12%, transparent);
    color: var(--color-danger);
  }

  .btn-icon.success:hover {
    background: var(--bg-success);
    color: var(--color-success);
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
    align-items: center;
  }

  .cards-wrapper {
    padding: 0 1.25rem 1rem 1.25rem;
  }

  .user-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 0.9rem;
    margin-top: 0.25rem;
  }

  .user-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
  }

  :global([data-theme='light']) .user-card {
    background: var(--bg-surface);
    border-color: var(--border-color);
    box-shadow: var(--shadow-sm);
  }

  .card-top {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: flex-start;
    padding: 1rem;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-surface);
    overflow: hidden;
  }

  :global([data-theme='light']) .card-top {
    border-bottom-color: var(--border-color);
    background: var(--bg-surface);
  }

  .user-meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .user-info {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    min-width: 0;
    flex: 1 1 0;
    overflow: hidden;
  }

  .user-name {
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .user-email {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    font-size: 0.85rem;
  }

  .card-bottom {
    padding: 1rem;
  }

  .meta-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.85rem 0.9rem;
  }

  .meta-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }

  .meta-label {
    font-size: 0.78rem;
    color: var(--text-secondary);
  }

  .meta-value {
    font-weight: 650;
    color: var(--text-primary);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cards-pagination {
    margin-top: 0.75rem;
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

  @media (max-width: 768px) {
    .cards-wrapper {
      padding: 0 1rem 1rem 1rem;
    }

    .user-cards {
      grid-template-columns: 1fr;
    }

    .card-top {
      padding: 0.85rem;
      gap: 0.5rem;
    }

    .card-bottom {
      padding: 0.85rem;
    }

    .actions {
      gap: 0.35rem;
    }

    .btn-icon {
      width: 32px;
      height: 32px;
      border-radius: 10px;
    }
  }

  @media (max-width: 480px) {
    .meta-grid {
      grid-template-columns: 1fr;
      gap: 0.65rem;
    }
  }
</style>