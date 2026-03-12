import { getTokenOrThrow, safeInvoke } from './core';
import type { PaginatedResponse } from './types';

export const mikrotik = {
  routers: {
    noc: (): Promise<any[]> => safeInvoke('list_mikrotik_noc', { token: getTokenOrThrow() }),
    list: (): Promise<any[]> => safeInvoke('list_mikrotik_routers', { token: getTokenOrThrow() }),
    get: (id: string): Promise<any> =>
      safeInvoke('get_mikrotik_router', { token: getTokenOrThrow(), id }),
    snapshot: (id: string): Promise<any> =>
      safeInvoke('get_mikrotik_router_snapshot', { token: getTokenOrThrow(), id }),
    create: (router: {
      name: string;
      host: string;
      port?: number;
      username: string;
      password: string;
      use_tls?: boolean;
      enabled?: boolean;
      maintenance_until?: string | null;
      maintenance_reason?: string | null;
      latitude?: number | null;
      longitude?: number | null;
    }): Promise<any> =>
      safeInvoke('create_mikrotik_router', {
        token: getTokenOrThrow(),
        name: router.name,
        host: router.host,
        port: router.port,
        username: router.username,
        password: router.password,
        use_tls: router.use_tls,
        useTls: router.use_tls,
        enabled: router.enabled,
        maintenance_until: router.maintenance_until,
        maintenanceUntil: router.maintenance_until,
        maintenance_reason: router.maintenance_reason,
        maintenanceReason: router.maintenance_reason,
        latitude: router.latitude,
        longitude: router.longitude,
      }),
    update: (
      id: string,
      router: {
        name?: string;
        host?: string;
        port?: number;
        username?: string;
        password?: string;
        use_tls?: boolean;
        enabled?: boolean;
        maintenance_until?: string | null;
        maintenance_reason?: string | null;
        latitude?: number | null;
        longitude?: number | null;
      },
    ): Promise<any> =>
      safeInvoke('update_mikrotik_router', {
        token: getTokenOrThrow(),
        id,
        name: router.name,
        host: router.host,
        port: router.port,
        username: router.username,
        password: router.password,
        use_tls: router.use_tls,
        useTls: router.use_tls,
        enabled: router.enabled,
        maintenance_until: router.maintenance_until ?? null,
        maintenanceUntil: router.maintenance_until ?? null,
        maintenance_reason: router.maintenance_reason ?? null,
        maintenanceReason: router.maintenance_reason ?? null,
        latitude: router.latitude ?? null,
        longitude: router.longitude ?? null,
      }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_mikrotik_router', { token: getTokenOrThrow(), id }),
    test: (id: string): Promise<any> =>
      safeInvoke('test_mikrotik_router', { token: getTokenOrThrow(), id }),
    metrics: (routerId: string, limit?: number): Promise<any[]> =>
      safeInvoke('list_mikrotik_router_metrics', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
        limit,
      }),
    interfaceMetrics: (
      routerId: string,
      params?: { interface?: string; limit?: number },
    ): Promise<any[]> =>
      safeInvoke('list_mikrotik_interface_metrics', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
        interface: params?.interface,
        limit: params?.limit,
      }),
    interfaceLatest: (routerId: string): Promise<any[]> =>
      safeInvoke('list_mikrotik_interface_latest', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
    interfaceLive: (routerId: string, names: string[]): Promise<any[]> =>
      safeInvoke('get_mikrotik_live_interface_counters', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
        names,
      }),
    pppProfiles: (routerId: string): Promise<any[]> =>
      safeInvoke('list_mikrotik_ppp_profiles', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
    syncPppProfiles: (routerId: string): Promise<any[]> =>
      safeInvoke('sync_mikrotik_ppp_profiles', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
    ipPools: (routerId: string): Promise<any[]> =>
      safeInvoke('list_mikrotik_ip_pools', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
    syncIpPools: (routerId: string): Promise<any[]> =>
      safeInvoke('sync_mikrotik_ip_pools', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
  },
  alerts: {
    list: (params?: { activeOnly?: boolean; limit?: number }): Promise<any[]> =>
      safeInvoke('list_mikrotik_alerts', {
        token: getTokenOrThrow(),
        active_only: params?.activeOnly,
        activeOnly: params?.activeOnly,
        limit: params?.limit,
      }),
    ack: (id: string): Promise<any> =>
      safeInvoke('ack_mikrotik_alert', { token: getTokenOrThrow(), id }),
    resolve: (id: string): Promise<any> =>
      safeInvoke('resolve_mikrotik_alert', { token: getTokenOrThrow(), id }),
  },
  incidents: {
    list: (params?: { activeOnly?: boolean; limit?: number }): Promise<any[]> =>
      safeInvoke('list_mikrotik_incidents', {
        token: getTokenOrThrow(),
        active_only: params?.activeOnly,
        activeOnly: params?.activeOnly,
        limit: params?.limit,
      }),
    ack: (id: string): Promise<any> =>
      safeInvoke('ack_mikrotik_incident', { token: getTokenOrThrow(), id }),
    resolve: (id: string): Promise<any> =>
      safeInvoke('resolve_mikrotik_incident', { token: getTokenOrThrow(), id }),
    update: (
      id: string,
      payload: { ownerUserId?: string | null; notes?: string | null },
    ): Promise<any> =>
      safeInvoke('update_mikrotik_incident', {
        token: getTokenOrThrow(),
        id,
        owner_user_id: payload.ownerUserId ?? null,
        ownerUserId: payload.ownerUserId ?? null,
        notes: payload.notes ?? null,
      }),
    simulate: (payload: {
      routerId: string;
      incidentType: string;
      severity?: string;
      interfaceName?: string | null;
      message?: string | null;
    }): Promise<any> =>
      safeInvoke('simulate_mikrotik_incident', {
        token: getTokenOrThrow(),
        router_id: payload.routerId,
        routerId: payload.routerId,
        incident_type: payload.incidentType,
        incidentType: payload.incidentType,
        severity: payload.severity,
        interface_name: payload.interfaceName ?? null,
        interfaceName: payload.interfaceName ?? null,
        message: payload.message ?? null,
      }),
    runAutoEscalation: (): Promise<{ ok: boolean; escalated: number }> =>
      safeInvoke('run_mikrotik_incident_auto_escalation', { token: getTokenOrThrow() }),
  },
  logs: {
    list: (params?: {
      routerId?: string;
      level?: string;
      topic?: string;
      q?: string;
      page?: number;
      perPage?: number;
      includeTotal?: boolean;
    }): Promise<PaginatedResponse<any>> =>
      safeInvoke('list_mikrotik_logs', {
        token: getTokenOrThrow(),
        router_id: params?.routerId,
        routerId: params?.routerId,
        level: params?.level,
        topic: params?.topic,
        q: params?.q,
        page: params?.page,
        per_page: params?.perPage,
        include_total: params?.includeTotal,
        includeTotal: params?.includeTotal,
      }),
    sync: (routerId: string, fetchLimit?: number): Promise<any> =>
      safeInvoke('sync_mikrotik_logs', {
        token: getTokenOrThrow(),
        router_id: routerId,
        routerId,
        fetch_limit: fetchLimit,
        fetchLimit,
      }),
  },
};
