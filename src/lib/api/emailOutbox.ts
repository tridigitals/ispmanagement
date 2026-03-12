import { getTokenOrThrow, safeInvoke } from './core';
import type { EmailOutboxItem, EmailOutboxStats, PaginatedResponse } from './types';

export const emailOutbox = {
  list: (params?: {
    scope?: 'tenant' | 'global' | 'all';
    page?: number;
    perPage?: number;
    status?: string;
    search?: string;
  }): Promise<PaginatedResponse<EmailOutboxItem>> =>
    safeInvoke('list_email_outbox', {
      token: getTokenOrThrow(),
      scope: params?.scope,
      page: params?.page,
      per_page: params?.perPage,
      status: params?.status,
      search: params?.search,
    }),

  get: (id: string): Promise<EmailOutboxItem> =>
    safeInvoke('get_email_outbox', { token: getTokenOrThrow(), id }),

  stats: (scope?: 'tenant' | 'global' | 'all'): Promise<EmailOutboxStats> =>
    safeInvoke('get_email_outbox_stats', { token: getTokenOrThrow(), scope }),

  retry: (id: string): Promise<void> =>
    safeInvoke('retry_email_outbox', { token: getTokenOrThrow(), id }),

  delete: (id: string): Promise<void> =>
    safeInvoke('delete_email_outbox', { token: getTokenOrThrow(), id }),

  retryBulk: (ids: string[]): Promise<{ success: boolean; count: number }> =>
    safeInvoke('bulk_retry_email_outbox', { token: getTokenOrThrow(), ids }),

  deleteBulk: (ids: string[]): Promise<{ success: boolean; count: number }> =>
    safeInvoke('bulk_delete_email_outbox', { token: getTokenOrThrow(), ids }),

  exportCsv: (params?: {
    scope?: 'tenant' | 'global' | 'all';
    status?: string;
    search?: string;
    limit?: number;
  }): Promise<{ csv: string }> =>
    safeInvoke('export_email_outbox_csv', {
      token: getTokenOrThrow(),
      scope: params?.scope,
      status: params?.status,
      search: params?.search,
      limit: params?.limit,
    }),
};
