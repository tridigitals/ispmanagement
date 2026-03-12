import { getApiBaseUrl } from '$lib/utils/apiUrl';
import { getTokenOrThrow, safeInvoke } from './core';
import type { FileRecord, PaginatedResponse } from './types';

export const storage = {
  listFiles: (
    page: number = 1,
    perPage: number = 20,
    search: string = '',
  ): Promise<PaginatedResponse<FileRecord>> =>
    safeInvoke('list_files_admin', {
      token: getTokenOrThrow(),
      page,
      perPage,
      search: search || null,
    }),

  deleteFile: (fileId: string): Promise<void> =>
    safeInvoke('delete_file_admin', { token: getTokenOrThrow(), fileId }),

  listFilesTenant: (
    page: number = 1,
    perPage: number = 20,
    search: string = '',
  ): Promise<PaginatedResponse<FileRecord>> =>
    safeInvoke('list_files_tenant', {
      token: getTokenOrThrow(),
      page,
      perPage,
      search: search || null,
    }),

  deleteFileTenant: (fileId: string): Promise<void> =>
    safeInvoke('delete_file_tenant', { token: getTokenOrThrow(), fileId }),

  uploadFile: async (
    file: File,
    options?: { paymentInvoiceId?: string | null },
  ): Promise<FileRecord> => {
    const apiBase = getApiBaseUrl();
    const formData = new FormData();
    formData.append('file', file);

    const query = new URLSearchParams();
    if (options?.paymentInvoiceId) {
      query.set('payment_invoice_id', options.paymentInvoiceId);
    }
    const url = `${apiBase}/storage/upload${query.toString() ? `?${query.toString()}` : ''}`;

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${getTokenOrThrow()}`,
      },
      body: formData,
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Upload failed: ${error}`);
    }

    return await response.json();
  },
};
