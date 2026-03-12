import { getTokenOrThrow, safeInvoke } from './core';
import type { AuditLog, PaginatedResponse } from './types';

export const audit = {
  listTenant: (
    page?: number,
    perPage?: number,
    filters?: {
      user_id?: string;
      customer_id?: string;
      resource?: string;
      resource_id?: string;
      action?: string;
      date_from?: string;
      date_to?: string;
      search?: string;
    },
  ): Promise<PaginatedResponse<AuditLog>> =>
    safeInvoke('list_tenant_audit_logs', { token: getTokenOrThrow(), page, perPage, ...filters }),
};
