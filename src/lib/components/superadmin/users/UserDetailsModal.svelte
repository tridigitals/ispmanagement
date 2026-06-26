<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';
  import { t } from 'svelte-i18n';
  import type { User } from '$lib/api/client';
  import { appSettings } from '$lib/stores/settings';
  import { formatDateTime } from '$lib/utils/date';

  let {
    show = $bindable(false),
    user,
    getTenantName,
  } = $props<{
    show: boolean;
    user: User | null;
    getTenantName: (u: any) => string;
  }>();

  function formatDateMaybe(value: any) {
    if (!value) return '-';
    const d = new Date(value);
    if (Number.isNaN(d.getTime())) return '-';
    return formatDateTime(d, { timeZone: $appSettings.app_timezone });
  }
</script>

<Modal
  bind:show
  title={user
    ? $t('superadmin.users.details.title_with_name', {
        values: { name: user.name },
      }) || `User Details — ${user.name}`
    : $t('superadmin.users.details.title') || 'User Details'}
  width="640px"
  onclose={() => {
    show = false;
    // Parent should handle clearing 'user' if needed, or we just close
  }}
>
  {#if user}
    <div class="details-grid">
      <div class="detail-card">
        <div class="detail-title">
          {$t('superadmin.users.details.sections.account')}
        </div>
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.name')}
          </span>
          <span class="detail-val">{user.name}</span>
        </div>
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.email')}
          </span>
          <span class="detail-val">{user.email}</span>
        </div>
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.role')}
          </span>
          <span class="detail-val">
            {#if user.is_super_admin}
              <span class="role-pill superadmin">{$t('sidebar.super_admin')}</span>
            {:else}
              <span class="role-pill {user.role}">{user.role}</span>
            {/if}
          </span>
        </div>
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.status')}
          </span>
          <span class="detail-val">
            {#if user.is_active}
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
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.created')}
          </span>
          <span class="detail-val">{formatDateMaybe(user.created_at)}</span>
        </div>
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.last_login')}
          </span>
          <span class="detail-val">
            {formatDateMaybe(
              (user as any).last_login_at ||
                (user as any).last_login ||
                (user as any).last_login_date,
            )}
          </span>
        </div>
      </div>

      <div class="detail-card">
        <div class="detail-title">
          {$t('superadmin.users.details.sections.tenant')}
        </div>
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.tenant')}
          </span>
          <span class="detail-val">
            {#if getTenantName(user as any)}
              {getTenantName(user as any)}
            {:else}
              -
            {/if}
          </span>
        </div>
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.slug')}
          </span>
          <span class="detail-val text-mono">
            {(user as any).tenant_slug || '-'}
          </span>
        </div>
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.tenant_role')}
          </span>
          <span class="detail-val">
            {(user as any).tenant_role || '-'}
          </span>
        </div>
      </div>

      <div class="detail-card">
        <div class="detail-title">
          {$t('superadmin.users.details.sections.security')}
        </div>
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.twofa_enabled')}
          </span>
          <span class="detail-val">
            {(user as any).two_factor_enabled ? $t('common.yes') || 'Yes' : $t('common.no') || 'No'}
          </span>
        </div>
        <div class="detail-row">
          <span class="detail-key">
            {$t('superadmin.users.details.labels.preferred_2fa')}
          </span>
          <span class="detail-val">
            {(user as any).preferred_2fa_method || '-'}
          </span>
        </div>
      </div>
    </div>
  {/if}
</Modal>

<style>
  .details-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 0.9rem;
  }

  @media (min-width: 720px) {
    .details-grid {
      grid-template-columns: 1fr 1fr;
    }

    .details-grid :global(.detail-card:nth-child(3)) {
      grid-column: 1 / -1;
    }
  }

  .detail-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 14px;
    padding: 1rem;
  }

  :global([data-theme='light']) .detail-card {
    background: var(--bg-surface);
    border-color: var(--border-color);
  }

  .detail-title {
    font-weight: 800;
    color: var(--text-primary);
    margin-bottom: 0.75rem;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    padding: 0.5rem 0;
    border-top: 1px solid var(--border-color);
  }

  :global([data-theme='light']) .detail-row {
    border-top-color: var(--border-color);
  }

  .detail-row:first-of-type {
    border-top: none;
    padding-top: 0;
  }

  .detail-key {
    color: var(--text-secondary);
    font-size: 0.9rem;
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

  .text-mono {
    font-family: monospace;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
</style>
