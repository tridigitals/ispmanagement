<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { can } from '$lib/stores/auth';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { t } from 'svelte-i18n';

  onMount(() => {
    if (
      !$can('read', 'backups') &&
      !$can('create', 'backups') &&
      !$can('download', 'backups') &&
      !$can('restore', 'backups') &&
      !$can('delete', 'backups')
    ) {
      goto('/unauthorized');
    }
  });
</script>

<div class="tenant-content">
  <div class="header-section">
    <div class="header-title">
      <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">
        {$t('sidebar.backups') || 'Backups'}
      </h1>
      <p class="text-sm text-gray-600 dark:text-gray-400">
        {$t('admin.backups.disabled_desc') ||
          'Backups are managed by Super Admin. Contact support if you need a restore.'}
      </p>
    </div>
  </div>

  <div class="glass-card empty-state">
    <div class="empty-icon">
      <Icon name="shield" size={22} />
    </div>
    <div class="empty-text">
      <h4>{$t('admin.backups.disabled_title') || 'Disabled for Tenants'}</h4>
      <p>
        {$t('admin.backups.disabled_body') ||
          'Tenant backup/restore is disabled to reduce security risk. Your data is covered by global backups.'}
      </p>
    </div>
  </div>
</div>

<style>
  .tenant-content {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1400px;
    margin: 0 auto;
  }

  .header-section {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 2rem;
    flex-wrap: wrap;
  }

  .header-title {
    display: grid;
    gap: 0.35rem;
  }

  .glass-card {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: 0 18px 45px rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(12px);
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
    box-shadow: 0 10px 20px rgba(0, 0, 0, 0.2);
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
</style>
