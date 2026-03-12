import { getTokenOrThrow, safeInvoke } from './core';
import type { AuditLog, PaginatedResponse } from './types';

export const superadmin = {
  listTenants: (): Promise<{ data: any[]; total: number }> =>
    safeInvoke('list_tenants', { token: getTokenOrThrow() }),

  createTenant: (
    name: string,
    slug: string,
    customDomain: string | null,
    ownerEmail: string,
    ownerPassword: string,
    planId?: string,
  ): Promise<any> =>
    safeInvoke('create_tenant', {
      token: getTokenOrThrow(),
      name,
      slug,
      customDomain,
      ownerEmail,
      ownerPassword,
      planId,
    }),

  deleteTenant: (id: string): Promise<void> =>
    safeInvoke('delete_tenant', { token: getTokenOrThrow(), id }),

  updateTenant: (
    id: string,
    name: string,
    slug: string,
    customDomain: string | null,
    isActive: boolean,
  ): Promise<any> =>
    safeInvoke('update_tenant', {
      token: getTokenOrThrow(),
      id,
      name,
      slug,
      customDomain,
      isActive,
    }),

  listAuditLogs: (
    page?: number,
    perPage?: number,
    filters?: {
      user_id?: string;
      tenant_id?: string;
      customer_id?: string;
      resource?: string;
      resource_id?: string;
      action?: string;
      date_from?: string;
      date_to?: string;
      search?: string;
    },
  ): Promise<PaginatedResponse<AuditLog>> =>
    safeInvoke('list_audit_logs', { token: getTokenOrThrow(), page, perPage, ...filters }),

  getSystemHealth: (): Promise<any> =>
    safeInvoke('get_system_health', { token: getTokenOrThrow() }),

  getSystemDiagnostics: (): Promise<any> =>
    safeInvoke('get_system_diagnostics', { token: getTokenOrThrow() }),
};
