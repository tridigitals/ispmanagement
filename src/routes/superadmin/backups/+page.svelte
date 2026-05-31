<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import type { BackupRecord } from '$lib/api/client';
  import { formatDate } from '$lib/utils/date';
  import { toast } from '$lib/stores/toast';
  import { appSettings } from '$lib/stores/settings';
  import Icon from '$lib/components/ui/Icon.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import { fly } from 'svelte/transition';
  import { t } from 'svelte-i18n';
  import { get } from 'svelte/store';

  let backups = $state<BackupRecord[]>([]);
  let loading = $state(false);
  let creating = $state(false);
  let restoring = $state(false);
  let isDeleting = $state(false);
  let downloading = $state<Record<string, number>>({}); // filename -> progress percent
  let fileInput: HTMLInputElement;
  let showDeleteModal = $state(false);
  let deleteTarget = $state<string | null>(null);
  let showRestoreModal = $state(false);
  let restoreMode = $state<'local' | 'upload' | null>(null);
  let restoreTarget = $state<string | null>(null);
  let restoreFile = $state<File | null>(null);

  async function loadBackups() {
    loading = true;
    try {
      backups = await api.backup.list({ scope: 'all' });
      backups = backups.filter((b) => b.backup_type === 'global');
    } catch (e: any) {
      toast.error(e.message);
    } finally {
      loading = false;
    }
  }

  async function createBackup() {
    creating = true;
    try {
      await api.backup.create('global');
      toast.success(
        get(t)('superadmin.backups.toast_create_success') || 'Global backup created successfully',
      );
      await loadBackups();
    } catch (e: any) {
      toast.error(e.message);
    } finally {
      creating = false;
    }
  }

  function requestDelete(filename: string) {
    deleteTarget = filename;
    showDeleteModal = true;
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    isDeleting = true;
    try {
      await api.backup.delete(deleteTarget);
      toast.success(get(t)('superadmin.backups.toast_delete_success') || 'Backup deleted');
      await loadBackups();
    } catch (e: any) {
      toast.error(e.message);
    } finally {
      isDeleting = false;
      showDeleteModal = false;
      deleteTarget = null;
    }
  }

  async function downloadBackup(filename: string) {
    try {
      downloading[filename] = 0;
      await api.backup.download(filename, (p) => {
        downloading[filename] = p;
      });
      toast.success(get(t)('superadmin.backups.toast_download_success') || 'Download complete');
    } catch (e: any) {
      toast.error(
        (get(t)('superadmin.backups.toast_download_failed') || 'Download failed') +
          ': ' +
          e.message,
      );
    } finally {
      delete downloading[filename];
    }
  }

  function requestRestoreLocal(filename: string) {
    restoreMode = 'local';
    restoreTarget = filename;
    restoreFile = null;
    showRestoreModal = true;
  }

  function requestRestoreUpload(file?: File) {
    restoreMode = 'upload';
    restoreFile = file ?? null;
    restoreTarget = file?.name || null;
    showRestoreModal = true;
  }

  async function confirmRestore() {
    if (!restoreMode) return;
    restoring = true;
    try {
      if (restoreMode === 'local' && restoreTarget) {
        await api.backup.restoreLocal(restoreTarget);
      } else if (restoreMode === 'upload') {
        await api.backup.restore(restoreFile || undefined);
      }
      toast.success(
        get(t)('superadmin.backups.toast_restore_success') ||
          'System restore completed successfully. The application will now reload.',
      );
      setTimeout(() => window.location.reload(), 2000);
    } catch (e: any) {
      toast.error(
        (get(t)('superadmin.backups.toast_restore_failed') || 'Restore failed') + ': ' + e.message,
      );
    } finally {
      restoring = false;
      showRestoreModal = false;
      restoreMode = null;
      restoreTarget = null;
      restoreFile = null;
      if (fileInput) fileInput.value = '';
    }
  }

  async function handleRestore(event?: Event) {
    const file = (event?.target as HTMLInputElement)?.files?.[0];
    // @ts-ignore
    const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

    if (!isTauri && !file) return;
    requestRestoreUpload(file);
  }

  function formatBytes(bytes: number, decimals = 2) {
    if (!+bytes) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
  }

  onMount(loadBackups);
</script>

<div class="superadmin-content fade-in">
  <div class="header-section">
    <div>
      <h1 class="page-title">{$t('superadmin.backups.title') || 'System Backups'}</h1>
      <p class="muted">
        {$t('superadmin.backups.subtitle') ||
          'Manage global database backups and tenant data exports.'}
      </p>
    </div>
    <div class="header-buttons">
      <input
        type="file"
        accept=".zip"
        class="hidden"
        bind:this={fileInput}
        onchange={handleRestore}
      />
      <button
        class="btn btn-secondary"
        disabled={restoring}
        onclick={() => {
          // @ts-ignore
          const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
          if (isTauri) requestRestoreUpload();
          else fileInput?.click();
        }}
      >
        {#if restoring}
          <span class="spinner-xs"></span>
          <span>{$t('superadmin.backups.restoring') || 'Restoring...'}</span>
        {:else}
          <Icon name="refresh-cw" size={18} />
          <span>{$t('superadmin.backups.restore_from_file') || 'Restore from File'}</span>
        {/if}
      </button>
      <button class="btn btn-primary" disabled={creating} onclick={createBackup}>
        {#if creating}
          <span class="spinner-xs"></span>
        {:else}
          <Icon name="plus" size={18} />
        {/if}
        <span>{$t('superadmin.backups.create_global') || 'Create Global Backup'}</span>
      </button>
    </div>
  </div>

  <div class="glass-card" in:fly={{ y: 20, delay: 80 }}>
    <div class="card-header glass">
      <div>
        <h3>{$t('superadmin.backups.available_title') || 'Available Backups'}</h3>
        <span class="muted">
          {$t('superadmin.backups.available_desc') || 'History of generated backup files'}
        </span>
      </div>
      <div class="header-actions">
        <span class="count-badge">
          {backups.length}
          {$t('superadmin.backups.files_count') || 'files'}
        </span>
      </div>
    </div>

    <div class="table-container">
      {#if loading && backups.length === 0}
        <div class="empty-state">
          <span class="spinner"></span>
          <p>{$t('superadmin.backups.loading') || 'Loading backups...'}</p>
        </div>
      {:else if backups.length === 0}
        <div class="empty-state fancy">
          <div class="empty-icon">
            <Icon name="archive" size={28} />
          </div>
          <div class="empty-text">
            <h4>{$t('superadmin.backups.empty_title') || 'No backups yet'}</h4>
            <p>
              {$t('superadmin.backups.empty_desc') ||
                'Create a global backup to protect system data.'}
            </p>
          </div>
          <div class="empty-actions">
            <button class="btn btn-primary" disabled={creating || restoring} onclick={createBackup}>
              <Icon name="plus" size={16} />
              <span>{$t('superadmin.backups.create_global') || 'Create Global Backup'}</span>
            </button>
          </div>
        </div>
      {:else}
        <table class="data-table">
          <thead>
            <tr>
              <th>{$t('superadmin.backups.col_filename') || 'Filename'}</th>
              <th>{$t('superadmin.backups.col_type') || 'Type'}</th>
              <th>{$t('superadmin.backups.col_size') || 'Size'}</th>
              <th class="nowrap">{$t('superadmin.backups.col_created_at') || 'Created At'}</th>
              <th class="text-right">{$t('common.actions') || 'Actions'}</th>
            </tr>
          </thead>
          <tbody>
            {#each backups as backup}
              <tr>
                <td class="font-medium">{backup.name}</td>
                <td>
                  <span
                    class="badge"
                    class:badge-blue={backup.backup_type === 'global'}
                    class:badge-green={backup.backup_type === 'tenant'}
                  >
                    {backup.backup_type}
                  </span>
                </td>
                <td>{formatBytes(backup.size)}</td>
                <td class="nowrap">
                  {formatDate(backup.created_at, { timeZone: $appSettings.app_timezone })}
                </td>
                <td class="text-right">
                  <div class="flex justify-end gap-2">
                    <button
                      class="btn-icon btn-primary-text"
                      onclick={() => downloadBackup(backup.name)}
                      title={$t('superadmin.backups.tooltip_download') || 'Download Backup'}
                    >
                      <Icon name="download" size={16} />
                    </button>
                    <button
                      class="btn-icon btn-warning-text"
                      disabled={restoring}
                      onclick={() => requestRestoreLocal(backup.name)}
                      title={$t('superadmin.backups.tooltip_restore') || 'Restore from this file'}
                    >
                      <Icon name="refresh-cw" size={16} />
                    </button>
                    <button
                      class="btn-icon btn-danger-text"
                      onclick={() => requestDelete(backup.name)}
                      title={$t('superadmin.backups.tooltip_delete') || 'Delete Backup'}
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
  title={$t('superadmin.backups.delete_title') || 'Delete Backup'}
  message={($t('superadmin.backups.delete_message') || 'Are you sure you want to delete {name}?').replace(
    '{name}',
    deleteTarget || $t('superadmin.backups.delete_message_fallback') || 'this backup',
  )}
  confirmText={$t('common.delete') || 'Delete'}
  type="danger"
  loading={isDeleting}
  onconfirm={confirmDelete}
  oncancel={() => {
    showDeleteModal = false;
    deleteTarget = null;
  }}
/>

<ConfirmDialog
  bind:show={showRestoreModal}
  title={$t('superadmin.backups.restore_title') || 'Restore Backup'}
  message={($t('superadmin.backups.restore_message') ||
    'Restore from {name}? This will OVERWRITE existing system data!').replace(
    '{name}',
    restoreTarget ||
      (restoreMode === 'upload'
        ? $t('superadmin.backups.restore_message_upload_fallback') || 'selected file'
        : $t('superadmin.backups.restore_message_local_fallback') || 'this backup'),
  )}
  confirmText={$t('superadmin.backups.restore_title') || 'Restore'}
  type="warning"
  loading={restoring}
  onconfirm={confirmRestore}
  oncancel={() => {
    showRestoreModal = false;
    restoreMode = null;
    restoreTarget = null;
    restoreFile = null;
  }}
/>

<style>
  .superadmin-content {
    padding: clamp(16px, 3vw, 32px);
    max-width: 1400px;
    margin: 0 auto;
  }

  .header-section {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .page-title {
    margin: 0 0 0.25rem 0;
    font-size: 1.5rem;
    font-weight: 800;
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }

  .header-buttons {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .glass-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
  }

  :global([data-theme='light']) .glass-card {
    background: var(--bg-surface);
    border-color: var(--border-color);
    box-shadow: var(--shadow-sm);
  }

  .card-header {
    padding: 1.25rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    border-bottom: 1px solid var(--border-color);
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
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    font-size: 0.85rem;
    font-weight: 650;
  }

  .table-container {
    overflow-x: auto;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    font-size: 0.9rem;
  }

  .data-table th {
    padding: 1rem 1.25rem;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    font-weight: 600;
    text-transform: uppercase;
    font-size: 0.75rem;
    letter-spacing: 0.05em;
  }

  .data-table td {
    padding: 1rem 1.25rem;
    border-top: 1px solid var(--border-color);
  }

  .data-table tr:hover {
    background: color-mix(in srgb, var(--color-primary) 4%, transparent);
  }

  .text-right {
    text-align: right;
  }

  .font-medium {
    font-weight: 500;
    color: var(--text-primary);
  }

  .nowrap {
    white-space: nowrap;
  }

  .badge {
    padding: 0.25rem 0.6rem;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .badge-blue {
    background: rgba(59, 130, 246, 0.1);
    color: #60a5fa;
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

  .empty-state.fancy {
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

  @media (max-width: 720px) {
    .header-section {
      align-items: stretch;
      margin-bottom: 1.25rem;
    }

    .header-section :global(.btn) {
      width: 100%;
      justify-content: center;
    }

    .card-header {
      align-items: flex-start;
    }

    .data-table th,
    .data-table td {
      padding: 0.85rem 0.95rem;
    }

    .text-right {
      text-align: left;
    }

    .justify-end {
      justify-content: flex-start;
    }

    .empty-state.fancy {
      padding: 1.5rem 1rem;
    }

    .empty-actions {
      width: 100%;
    }

    .empty-actions :global(.btn) {
      width: 100%;
      justify-content: center;
    }
  }
</style>
