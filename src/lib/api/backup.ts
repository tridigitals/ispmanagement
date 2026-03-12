import { invoke } from '@tauri-apps/api/core';
import { getApiBaseUrl } from '$lib/utils/apiUrl';
import { getTokenOrThrow, isTauriRuntime, safeInvoke } from './core';
import type { BackupRecord } from './types';

export const backup = {
  list: async (opts?: { scope?: 'all' | 'tenant' }): Promise<BackupRecord[]> => {
    const isTauri = isTauriRuntime();
    const token = getTokenOrThrow();
    const tenantOnly = opts?.scope === 'tenant';
    if (isTauri) {
      return await invoke('list_backups', { args: { token, tenantOnly } });
    }
    return await safeInvoke('list_backups', { token, tenantOnly });
  },

  create: async (backupType: 'global' | 'tenant', targetId?: string): Promise<string> => {
    const isTauri = isTauriRuntime();
    const token = getTokenOrThrow();
    if (isTauri) {
      return await invoke('create_backup', { args: { token, backupType, targetId } });
    }
    return await safeInvoke('create_backup', { token, backupType, targetId });
  },

  delete: async (filename: string): Promise<void> => {
    const isTauri = isTauriRuntime();
    const token = getTokenOrThrow();
    if (isTauri) {
      await invoke('delete_backup', { args: { token, filename } });
      return;
    }
    await safeInvoke('delete_backup', { token, filename });
  },

  download: async (filename: string, onProgress?: (percent: number) => void): Promise<void> => {
    const isTauri = isTauriRuntime();
    const token = getTokenOrThrow();
    const apiBase = getApiBaseUrl();
    const url = `${apiBase}/backups/${filename}/download`;

    try {
      const response = await fetch(url, {
        headers: { Authorization: `Bearer ${token}` },
      });

      if (!response.ok) throw new Error(`Download failed: ${response.statusText}`);

      const contentLength = response.headers.get('content-length');
      const total = contentLength ? parseInt(contentLength, 10) : 0;
      let loaded = 0;

      const reader = response.body?.getReader();
      if (!reader) throw new Error('Response body is null');

      const chunks = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
        loaded += value.length;
        if (total > 0 && onProgress) {
          onProgress(Math.round((loaded / total) * 100));
        }
      }

      const blob = new Blob(chunks);

      if (isTauri) {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const { writeFile } = await import('@tauri-apps/plugin-fs');

        const filePath = await save({
          defaultPath: filename,
          filters: [{ name: 'Archive', extensions: ['zip', 'sql'] }],
        });

        if (filePath) {
          await writeFile(filePath, new Uint8Array(await blob.arrayBuffer()));
        }
      } else {
        const downloadUrl = window.URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = downloadUrl;
        link.setAttribute('download', filename);
        document.body.appendChild(link);
        link.click();
        link.remove();
        window.URL.revokeObjectURL(downloadUrl);
      }
    } catch (e: any) {
      console.error('Download error:', e);
      throw e;
    }
  },

  restore: async (file?: File): Promise<void> => {
    const isTauri = isTauriRuntime();
    const token = getTokenOrThrow();

    if (isTauri && !file) {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        filters: [{ name: 'Archive', extensions: ['zip'] }],
      });

      if (selected && typeof selected === 'string') {
        return await safeInvoke('restore_backup_from_file', { token, path: selected });
      }
      throw new Error('No file selected');
    } else if (file) {
      const apiBase = getApiBaseUrl();
      const formData = new FormData();
      formData.append('file', file);

      const response = await fetch(`${apiBase}/backups/restore`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${token}` },
        body: formData,
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.error || 'Restore failed');
      }
    } else {
      throw new Error('File required for web restore');
    }
  },

  restoreLocal: async (filename: string): Promise<void> => {
    const token = getTokenOrThrow();
    return await safeInvoke('restore_local_backup_command', { token, filename });
  },
};
