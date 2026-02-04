import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import { isTauri } from '@tauri-apps/api/core';
import { toast } from 'svelte-sonner';
import { t } from 'svelte-i18n';
import { get } from 'svelte/store';

/**
 * Downloads a file.
 * In Tauri desktop environment, it uses the native "Save As" dialog and filesystem.
 * In Web environment, it uses the standard browser download behavior.
 *
 * @param url The direct URL to the file content (e.g. /api/storage/files/123/download or /content)
 * @param filename The suggested filename
 */
export async function downloadFile(url: string, filename: string) {
  if (isTauri()) {
    try {
      // 1. Open Native Save Dialog
      const savePath = await save({
        defaultPath: filename,
        title: get(t)('utils.download.save_title') || 'Save File',
      });

      if (!savePath) return; // User cancelled

      const loadingToast = toast.loading(
        get(t)('utils.download.downloading', {
          values: { filename },
        }) || `Downloading ${filename}...`,
      );

      // 2. Fetch the file content as binary
      // Note: We use standard fetch here. If you have authentication cookies/headers,
      // ensure they are sent (credentials: 'include').
      const response = await fetch(url, {
        headers: {
          // Add auth headers if needed, or rely on cookie
        },
      });

      if (!response.ok)
        throw new Error(get(t)('utils.download.failed_generic') || 'Download failed');

      const blob = await response.blob();
      const buffer = await blob.arrayBuffer();
      const uint8Array = new Uint8Array(buffer);

      // 3. Write to disk
      await writeFile(savePath, uint8Array);

      toast.dismiss(loadingToast);
      toast.success(get(t)('common.file_saved') || 'File saved successfully');
    } catch (e: any) {
      console.error('Download error:', e);
      toast.error(
        get(t)('utils.download.failed', {
          values: { message: e?.message || e },
        }) || `Download failed: ${e.message}`,
      );
    }
  } else {
    // Web Fallback: Standard <a> tag download
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    link.target = '_blank'; // Safe fallback
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  }
}
