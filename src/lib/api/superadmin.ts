import { getTokenOrThrow, safeInvoke } from './core';
import type {
  AuditLog,
  ManagedRadiusAssignmentPayload,
  ManagedRadiusMappingPayload,
  ManagedRadiusSecretValue,
  ManagedRadiusServerPayload,
  PaginatedResponse,
  SuperadminManagedRadiusAssignment,
  SuperadminManagedRadiusMapping,
  SuperadminManagedRadiusRuntimeStatus,
  SuperadminManagedRadiusSession,
  SuperadminManagedRadiusServer,
  SuperadminManagedRadiusUser,
} from './types';

export const superadmin = {
  listTenants: (): Promise<{ data: any[]; total: number }> =>
    safeInvoke('list_tenants', { token: getTokenOrThrow() }),

  getManagedRadiusRuntimeStatus: (): Promise<SuperadminManagedRadiusRuntimeStatus> =>
    safeInvoke('get_managed_radius_runtime_status', { token: getTokenOrThrow() }),

  listManagedRadiusServers: (): Promise<{ data: SuperadminManagedRadiusServer[]; total: number }> =>
    safeInvoke('list_managed_radius_servers', { token: getTokenOrThrow() }),

  createManagedRadiusServer: (payload: ManagedRadiusServerPayload): Promise<{ ok: boolean; id: string }> =>
    safeInvoke('create_managed_radius_server', {
      token: getTokenOrThrow(),
      name: payload.name,
      endpointHost: payload.endpoint_host,
      endpoint_host: payload.endpoint_host,
      endpointPort: payload.endpoint_port ?? null,
      endpoint_port: payload.endpoint_port ?? null,
      runtimeLabel: payload.runtime_label ?? null,
      runtime_label: payload.runtime_label ?? null,
      runtimeUser: payload.runtime_user ?? null,
      runtime_user: payload.runtime_user ?? null,
      runtimeSecret: payload.runtime_secret ?? null,
      runtime_secret: payload.runtime_secret ?? null,
      isActive: payload.is_active,
      is_active: payload.is_active,
      notes: payload.notes ?? null,
    }),

  updateManagedRadiusServer: (
    id: string,
    payload: ManagedRadiusServerPayload,
  ): Promise<{ ok: boolean }> =>
    safeInvoke('update_managed_radius_server', {
      token: getTokenOrThrow(),
      id,
      name: payload.name,
      endpointHost: payload.endpoint_host,
      endpoint_host: payload.endpoint_host,
      endpointPort: payload.endpoint_port ?? null,
      endpoint_port: payload.endpoint_port ?? null,
      runtimeLabel: payload.runtime_label ?? null,
      runtime_label: payload.runtime_label ?? null,
      runtimeUser: payload.runtime_user ?? null,
      runtime_user: payload.runtime_user ?? null,
      runtimeSecret: payload.runtime_secret ?? null,
      runtime_secret: payload.runtime_secret ?? null,
      isActive: payload.is_active,
      is_active: payload.is_active,
      notes: payload.notes ?? null,
    }),

  setManagedRadiusServerActive: (id: string, isActive: boolean): Promise<{ ok: boolean }> =>
    safeInvoke('set_managed_radius_server_active', {
      token: getTokenOrThrow(),
      id,
      isActive,
      is_active: isActive,
    }),

  setManagedRadiusServerDefault: (id: string): Promise<{ ok: boolean }> =>
    safeInvoke('set_managed_radius_server_default', {
      token: getTokenOrThrow(),
      id,
    }),

  listManagedRadiusAssignments: (): Promise<{ data: SuperadminManagedRadiusAssignment[]; total: number }> =>
    safeInvoke('list_managed_radius_assignments', { token: getTokenOrThrow() }),

  createManagedRadiusAssignment: (
    payload: ManagedRadiusAssignmentPayload,
  ): Promise<{ ok: boolean; id: string }> =>
    safeInvoke('create_managed_radius_assignment', {
      token: getTokenOrThrow(),
      tenantId: payload.tenant_id,
      tenant_id: payload.tenant_id,
      radiusServerId: payload.radius_server_id,
      radius_server_id: payload.radius_server_id,
      isActive: payload.is_active,
      is_active: payload.is_active,
    }),

  updateManagedRadiusAssignment: (
    id: string,
    payload: ManagedRadiusAssignmentPayload,
  ): Promise<{ ok: boolean }> =>
    safeInvoke('update_managed_radius_assignment', {
      token: getTokenOrThrow(),
      id,
      tenantId: payload.tenant_id,
      tenant_id: payload.tenant_id,
      radiusServerId: payload.radius_server_id,
      radius_server_id: payload.radius_server_id,
      isActive: payload.is_active,
      is_active: payload.is_active,
    }),

  setManagedRadiusAssignmentActive: (
    id: string,
    tenantId: string,
    isActive: boolean,
  ): Promise<{ ok: boolean }> =>
    safeInvoke('set_managed_radius_assignment_active', {
      token: getTokenOrThrow(),
      id,
      tenantId,
      tenant_id: tenantId,
      isActive,
      is_active: isActive,
    }),

  listManagedRadiusMappings: (): Promise<{ data: SuperadminManagedRadiusMapping[]; total: number }> =>
    safeInvoke('list_managed_radius_mappings', { token: getTokenOrThrow() }),

  createManagedRadiusMapping: (payload: ManagedRadiusMappingPayload): Promise<{ ok: boolean; id: string }> =>
    safeInvoke('create_managed_radius_mapping', {
      token: getTokenOrThrow(),
      tenantId: payload.tenant_id,
      tenant_id: payload.tenant_id,
      radiusServerId: payload.radius_server_id,
      radius_server_id: payload.radius_server_id,
      routerId: payload.router_id,
      router_id: payload.router_id,
      nasName: payload.nas_name,
      nas_name: payload.nas_name,
      nasIpOrCidr: payload.nas_ip_or_cidr,
      nas_ip_or_cidr: payload.nas_ip_or_cidr,
      shortname: payload.shortname ?? null,
      sharedSecret: payload.shared_secret ?? null,
      shared_secret: payload.shared_secret ?? null,
      isActive: payload.is_active,
      is_active: payload.is_active,
    }),

  updateManagedRadiusMapping: (
    id: string,
    payload: ManagedRadiusMappingPayload,
  ): Promise<{ ok: boolean }> =>
    safeInvoke('update_managed_radius_mapping', {
      token: getTokenOrThrow(),
      id,
      tenantId: payload.tenant_id,
      tenant_id: payload.tenant_id,
      radiusServerId: payload.radius_server_id,
      radius_server_id: payload.radius_server_id,
      routerId: payload.router_id,
      router_id: payload.router_id,
      nasName: payload.nas_name,
      nas_name: payload.nas_name,
      nasIpOrCidr: payload.nas_ip_or_cidr,
      nas_ip_or_cidr: payload.nas_ip_or_cidr,
      shortname: payload.shortname ?? null,
      sharedSecret: payload.shared_secret ?? null,
      shared_secret: payload.shared_secret ?? null,
      isActive: payload.is_active,
      is_active: payload.is_active,
    }),

  setManagedRadiusMappingActive: (
    id: string,
    tenantId: string,
    isActive: boolean,
  ): Promise<{ ok: boolean }> =>
    safeInvoke('set_managed_radius_mapping_active', {
      token: getTokenOrThrow(),
      id,
      tenantId,
      tenant_id: tenantId,
      isActive,
      is_active: isActive,
    }),

  rotateManagedRadiusMappingSecret: (
    id: string,
    tenantId: string,
    sharedSecret?: string | null,
  ): Promise<ManagedRadiusSecretValue> =>
    safeInvoke('rotate_managed_radius_mapping_secret', {
      token: getTokenOrThrow(),
      id,
      tenantId,
      tenant_id: tenantId,
      sharedSecret: sharedSecret ?? null,
      shared_secret: sharedSecret ?? null,
    }),

  revealManagedRadiusMappingSecret: (
    id: string,
    tenantId: string,
  ): Promise<ManagedRadiusSecretValue> =>
    safeInvoke('reveal_managed_radius_mapping_secret', {
      token: getTokenOrThrow(),
      id,
      tenantId,
      tenant_id: tenantId,
    }),

  listManagedRadiusUsers: (): Promise<{ data: SuperadminManagedRadiusUser[]; total: number }> =>
    safeInvoke('list_managed_radius_users', { token: getTokenOrThrow() }),

  listManagedRadiusSessions: (): Promise<{ data: SuperadminManagedRadiusSession[]; total: number }> =>
    safeInvoke('list_managed_radius_sessions', { token: getTokenOrThrow() }),

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
