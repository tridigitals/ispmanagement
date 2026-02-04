<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { BackupRecord } from '$lib/api/client';
  import { formatDate } from '$lib/utils/date';
  import { toast } from '$lib/stores/toast';
  import { appSettings } from '$lib/stores/settings';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { fly } from 'svelte/transition';
  import { t } from 'svelte-i18n';
  import { can, tenant, user } from '$lib/stores/auth';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';

  let backups = $state<BackupRecord[]>([]);
  let loading = $state(false);
  let creating = $state(false);
  let restoring = $state(false);
  let downloading = $state<Record<string, number>>({});
  let fileInput: HTMLInputElement;
  let showDeleteModal = $state(false);
  let showRestoreModal = $state(false);
  let deleteTarget = $state<string | null>(null);
  let restoreTarget = $state<string | null>(null);
  let restoringFromUpload = $state(false);

  const canRead = $derived(
    ($user?.is_super_admin ?? false) || $user?.role === 'Owner' || $can('read', 'backups'),
  );
  const canCreate = $derived(
    ($user?.is_super_admin ?? false) || $user?.role === 'Owner' || $can('create', 'backups'),
  );
  const canDownload = $derived(
    ($user?.is_super_admin ?? false) || $user?.role === 'Owner' || $can('download', 'backups'),
  );
  const canRestore = $derived(
    ($user?.is_super_admin ?? false) || $user?.role === 'Owner' || $can('restore', 'backups'),
  );
  const canDelete = $derived(
    ($user?.is_super_admin ?? false) || $user?.role === 'Owner' || $can('delete', 'backups'),
  );

  async function loadBackups() {
    if (!canRead) return;
    loading = true;
    try {
      backups = await api.backup.list({ scope: 'tenant' });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load backups');
    } finally {
      loading = false;
    }
  }

  async function createBackup() {
    if (!canCreate) {
      toast.error('You do not have permission to create backups');
      return;
    }

    const tenantId = $user?.tenant_id || $tenant?.id;
    if (!tenantId) {
      toast.error('Missing tenant context');
      return;
    }

    creating = true;
    try {
      await api.backup.create('tenant', tenantId);
      toast.success('Backup created successfully');
      await loadBackups();
    } catch (e: any) {
      console.error('Create backup failed:', e);
      toast.error(e?.message || 'Create backup failed');
    } finally {
      creating = false;
    }
  }

  async function deleteBackup(filename: string) {
    if (!canDelete) {
      toast.error('You do not have permission to delete backups');
      return;
    }
    deleteTarget = filename;
    showDeleteModal = true;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    try {
      await api.backup.delete(deleteTarget);
      toast.success('Backup deleted');
      await loadBackups();
    } catch (e: any) {
      toast.error(e?.message || 'Delete failed');
    } finally {
      showDeleteModal = false;
      deleteTarget = null;
    }
  }

  async function downloadBackup(filename: string) {
    if (!canDownload) {
      toast.error('You do not have permission to download backups');
      return;
    }
    try {
      downloading[filename] = 0;
      await api.backup.download(filename, (p) => {
        downloading[filename] = p;
      });
      toast.success('Download complete');
    } catch (e: any) {
      toast.error('Download failed: ' + (e?.message || 'Unknown error'));
    } finally {
      delete downloading[filename];
    }
  }

  async function restoreLocal(filename: string) {
    if (!canRestore) {
      toast.error('You do not have permission to restore backups');
      return;
    }
    restoreTarget = filename;
    restoringFromUpload = false;
    showRestoreModal = true;
  }

  async function confirmRestore() {
    if (!canRestore) return;
    restoring = true;
    try {
      if (restoringFromUpload) {
        await api.backup.restore();
      } else if (restoreTarget) {
        await api.backup.restoreLocal(restoreTarget);
      } else {
        return;
      }
      toast.success('Restore completed successfully. Reloading...');
      setTimeout(() => window.location.reload(), 2000);
    } catch (e: any) {
      toast.error('Restore failed: ' + (e?.message || 'Unknown error'));
    } finally {
      restoring = false;
      showRestoreModal = false;
      restoreTarget = null;
    }
  }

  async function handleRestore(event?: Event) {
    if (!canRestore) {
      toast.error('You do not have permission to restore backups');
      return;
    }
    const file = (event?.target as HTMLInputElement)?.files?.[0];
    // @ts-ignore
    const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

    if (!isTauri && !file) return;

    // For web: confirm after file picked. For Tauri: confirm then open picker.
    restoringFromUpload = true;
    restoreTarget = file?.name || null;
    showRestoreModal = true;
  }

  function formatBytes(bytes: number, decimals = 2) {
    if (!+bytes) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
  }

  onMount(loadBackups);
</script>

<div class="tenant-content fade-in">
  <div class="header-section">
    <div>
      <h1 class="text-2xl font-semibold text-gray-900 dark:text-white">
        {$t('sidebar.backups') || 'Backups'}
      </h1>
      <p class="muted">Export and restore your organization data.</p>
    </div>
    <div class="flex gap-2 header-actions">
      <input
        type="file"
        accept=".zip"
        class="hidden"
        bind:this={fileInput}
        onchange={handleRestore}
      />

      <button
        class="btn btn-secondary"
        disabled={restoring || !canRestore}
        onclick={() => {
          // @ts-ignore
          const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
          if (isTauri) handleRestore();
          else fileInput?.click();
        }}
        title={!canRestore ? 'Missing permission backups:restore' : undefined}
      >
        <Icon name="refresh-cw" size={16} />
        <span>Restore from File</span>
      </button>

      <button
        class="btn btn-primary"
        disabled={creating || restoring || !canCreate}
        onclick={createBackup}
        title={!canCreate ? 'Missing permission backups:create' : undefined}
      >
        <Icon name="plus" size={16} />
        <span>Create Tenant Backup</span>
      </button>
    </div>
  </div>

  <div class="glass-card" in:fly={{ y: 20, delay: 80 }}>
    <div class="card-header">
      <div>
        <h3>Available Backups</h3>
        <p class="muted">History of generated backup files</p>
      </div>
      <span class="count-badge">{backups.length} files</span>
    </div>

    <div class="table-container">
      {#if loading}
        <div class="p-4 muted">Loading backups...</div>
      {:else if !canRead}
        <div class="p-4 muted">You do not have permission to view backups.</div>
      {:else if backups.length === 0}
        <div class="empty-state">
          <div class="empty-icon">
            <Icon name="archive" size={28} />
          </div>
          <div class="empty-text">
            <h4>No backups yet</h4>
            <p>Create your first tenant backup to keep your data safe.</p>
          </div>
          <div class="empty-actions">
            <button
              class="btn btn-primary"
              disabled={creating || restoring || !canCreate}
              onclick={createBackup}
            >
              <Icon name="plus" size={16} />
              <span>Create Tenant Backup</span>
            </button>
          </div>
        </div>
      {:else}
        <table class="data-table">
          <thead>
            <tr>
              <th>Filename</th>
              <th>Type</th>
              <th>Size</th>
              <th class="nowrap">Created At</th>
              <th class="text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each backups as backup (backup.name)}
              <tr>
                <td class="font-medium filename-cell">{backup.name}</td>
                <td>
                  <span class="badge badge-green">TENANT</span>
                </td>
                <td>{formatBytes(backup.size)}</td>
                <td class="nowrap">
                  {formatDate(backup.created_at, { timeZone: $appSettings.app_timezone })}
                </td>
                <td class="text-right">
                  <div class="flex justify-end gap-2">
                    <button
                      class="btn-icon btn-primary-text"
                      disabled={restoring || !canDownload}
                      onclick={() => downloadBackup(backup.name)}
                      title="Download Backup"
                    >
                      <Icon name="download" size={16} />
                    </button>
                    <button
                      class="btn-icon btn-warning-text"
                      disabled={restoring || !canRestore}
                      onclick={() => restoreLocal(backup.name)}
                      title="Restore from this file"
                    >
                      <Icon name="refresh-cw" size={16} />
                    </button>
                    <button
                      class="btn-icon btn-danger-text"
                      disabled={!canDelete}
                      onclick={() => deleteBackup(backup.name)}
                      title="Delete Backup"
                    >
                      <Icon name="trash" size={16} />
                    </button>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </div>
</div>

<ConfirmDialog
  bind:show={showDeleteModal}
  title="Delete Backup"
  message={`Are you sure you want to delete ${deleteTarget || 'this backup'}?`}
  confirmText="Delete"
  type="danger"
  loading={restoring}
  onconfirm={confirmDelete}
/>

<ConfirmDialog
  bind:show={showRestoreModal}
  title="Restore Backup"
  message={restoringFromUpload
    ? `Restore from ${restoreTarget || 'selected file'}? This will REPLACE your organization data.`
    : `Restore from ${restoreTarget || 'this backup'}? This will REPLACE your organization data.`}
  confirmText="Restore"
  type="warning"
  loading={restoring}
  onconfirm={confirmRestore}
/>

<style>
  .tenant-content {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1400px;
    margin: 0 auto;
  }

  .header-section {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    margin-bottom: 2rem;
    flex-wrap: wrap;
  }

  .header-actions {
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .glass-card {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: 0 18px 45px rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(12px);
  }

  .card-header {
    padding: 1.25rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    gap: 1rem;
  }

  .card-header h3 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 800;
  }

  .muted {
    color: var(--text-secondary);
    font-size: 0.9rem;
  }

  .count-badge {
    background: rgba(255, 255, 255, 0.05);
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    font-size: 0.85rem;
    white-space: nowrap;
  }

  .table-container {
    overflow-x: auto;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    font-size: 0.9rem;
    min-width: 920px;
  }

  .data-table th {
    padding: 1rem 1.25rem;
    background: rgba(255, 255, 255, 0.02);
    color: var(--text-secondary);
    font-weight: 600;
    text-transform: uppercase;
    font-size: 0.75rem;
    letter-spacing: 0.05em;
  }

  .data-table td {
    padding: 1rem 1.25rem;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
    vertical-align: middle;
  }

  .data-table tr:hover {
    background: rgba(255, 255, 255, 0.01);
  }

  .filename-cell {
    max-width: 520px;
    word-break: break-word;
    overflow-wrap: anywhere;
  }

  .nowrap {
    white-space: nowrap;
  }

  .text-right {
    text-align: right;
  }

  .font-medium {
    font-weight: 500;
    color: var(--text-primary);
  }

  .badge {
    padding: 0.25rem 0.6rem;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .badge-green {
    background: rgba(16, 185, 129, 0.1);
    color: #34d399;
  }

  .btn-danger-text {
    color: var(--color-danger);
    background: transparent;
    border: none;
  }

  .btn-danger-text:hover {
    background: rgba(239, 68, 68, 0.1);
  }

  .btn-primary-text {
    color: var(--color-primary);
    background: transparent;
    border: none;
  }

  .btn-primary-text:hover {
    background: var(--color-primary-subtle);
  }

  .btn-warning-text {
    color: var(--color-warning);
    background: transparent;
    border: none;
  }

  .btn-warning-text:hover {
    background: rgba(245, 158, 11, 0.1);
  }

  .flex {
    display: flex;
  }
  .justify-end {
    justify-content: flex-end;
  }
  .gap-2 {
    gap: 0.5rem;
  }
  .hidden {
    display: none;
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

  .empty-actions {
    display: flex;
    justify-content: center;
    gap: 0.75rem;
  }

  @media (max-width: 640px) {
    .header-actions {
      width: 100%;
      justify-content: stretch;
    }

    :global(.btn.btn-primary),
    :global(.btn.btn-secondary) {
      width: 100%;
      justify-content: center;
    }

    .data-table {
      min-width: 760px;
    }

    .empty-state {
      padding: 2rem 1.25rem;
    }

    .empty-actions .btn {
      width: 100%;
      justify-content: center;
    }
  }
</style>
