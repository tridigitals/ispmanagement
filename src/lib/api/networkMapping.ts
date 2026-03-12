import { getTokenOrThrow, safeInvoke } from './core';
import type { PaginatedResponse } from './types';

type JsonGeometry = {
  type: string;
  coordinates?: unknown;
  geometries?: unknown[];
};

export const networkMapping = {
  nodes: {
    list: (
      params?: {
        q?: string;
        page?: number;
        per_page?: number;
        status?: string;
        kind?: string;
        bbox?: string;
      },
      options?: { signal?: AbortSignal },
    ): Promise<PaginatedResponse<any>> =>
      safeInvoke('list_network_nodes', {
        token: getTokenOrThrow(),
        ...(params || {}),
        __signal: options?.signal,
      }),
    create: (dto: any): Promise<any> =>
      safeInvoke('create_network_node', { token: getTokenOrThrow(), ...dto }),
    update: (id: string, dto: any): Promise<any> =>
      safeInvoke('update_network_node', { token: getTokenOrThrow(), id, ...dto }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_network_node', { token: getTokenOrThrow(), id }),
  },
  links: {
    list: (
      params?: {
        q?: string;
        page?: number;
        per_page?: number;
        status?: string;
        kind?: string;
        bbox?: string;
      },
      options?: { signal?: AbortSignal },
    ): Promise<PaginatedResponse<any>> =>
      safeInvoke('list_network_links', {
        token: getTokenOrThrow(),
        ...(params || {}),
        __signal: options?.signal,
      }),
    create: (dto: any): Promise<any> =>
      safeInvoke('create_network_link', { token: getTokenOrThrow(), ...dto }),
    connectNodeToLink: (dto: {
      source_node_id: string;
      target_link_id: string;
      name: string;
      link_type: string;
      status?: string;
      priority?: number;
      capacity_mbps?: number | null;
      utilization_pct?: number | null;
      loss_db?: number | null;
      latency_ms?: number | null;
      geometry: JsonGeometry;
      split_lat: number;
      split_lng: number;
      junction_name?: string;
      junction_node_type?: string;
      metadata?: Record<string, any>;
    }): Promise<any> =>
      safeInvoke('connect_network_node_to_link', { token: getTokenOrThrow(), ...dto }),
    update: (id: string, dto: any): Promise<any> =>
      safeInvoke('update_network_link', { token: getTokenOrThrow(), id, ...dto }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_network_link', { token: getTokenOrThrow(), id }),
  },
  paths: {
    compute: (dto: {
      source_node_id: string;
      target_node_id: string;
      max_hops?: number;
      max_utilization_pct?: number;
      allowed_link_types?: string[];
      allowed_statuses?: string[];
      exclude_link_ids?: string[];
      require_active_nodes?: boolean;
    }): Promise<any> =>
      safeInvoke('compute_network_path', { token: getTokenOrThrow(), ...dto }),
  },
  assets: {
    sync: (): Promise<{
      router_nodes_created: number;
      router_nodes_updated: number;
      customer_nodes_created: number;
      customer_nodes_updated: number;
      total_nodes_touched: number;
    }> => safeInvoke('sync_network_mapping_assets', { token: getTokenOrThrow() }),
  },
  candidates: {
    rank: (dto: {
      lat?: number;
      lng?: number;
      zone_id?: string;
      node_types?: string[];
      limit?: number;
      require_active_nodes?: boolean;
    }): Promise<any> =>
      safeInvoke('rank_candidate_network_nodes', { token: getTokenOrThrow(), ...dto }),
  },
  zones: {
    list: (
      params?: {
        q?: string;
        page?: number;
        per_page?: number;
        status?: string;
        kind?: string;
        bbox?: string;
      },
      options?: { signal?: AbortSignal },
    ): Promise<PaginatedResponse<any>> =>
      safeInvoke('list_service_zones', {
        token: getTokenOrThrow(),
        ...(params || {}),
        __signal: options?.signal,
      }),
    create: (dto: any): Promise<any> =>
      safeInvoke('create_service_zone', { token: getTokenOrThrow(), ...dto }),
    update: (id: string, dto: any): Promise<any> =>
      safeInvoke('update_service_zone', { token: getTokenOrThrow(), id, ...dto }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_service_zone', { token: getTokenOrThrow(), id }),
    resolve: (dto: { lat: number; lng: number }): Promise<any> =>
      safeInvoke('resolve_service_zone', { token: getTokenOrThrow(), ...dto }),
    checkCoverage: (dto: { lat: number; lng: number }): Promise<any> =>
      safeInvoke('check_network_coverage', { token: getTokenOrThrow(), ...dto }),
  },
  zoneOffers: {
    list: (params?: { zone_id?: string; package_id?: string; active_only?: boolean }): Promise<any[]> =>
      safeInvoke('list_zone_offers', { token: getTokenOrThrow(), ...(params || {}) }),
    create: (dto: any): Promise<any> =>
      safeInvoke('create_zone_offer', { token: getTokenOrThrow(), ...dto }),
    update: (id: string, dto: any): Promise<any> =>
      safeInvoke('update_zone_offer', { token: getTokenOrThrow(), id, ...dto }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_zone_offer', { token: getTokenOrThrow(), id }),
  },
  zoneBindings: {
    list: (params?: { zone_id?: string }): Promise<any[]> =>
      safeInvoke('list_zone_node_bindings', { token: getTokenOrThrow(), ...(params || {}) }),
    create: (dto: any): Promise<any> =>
      safeInvoke('create_zone_node_binding', { token: getTokenOrThrow(), ...dto }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_zone_node_binding', { token: getTokenOrThrow(), id }),
  },
  impact: {
    listCustomers: (params?: { node_id?: string; link_id?: string; router_id?: string }): Promise<any> =>
      safeInvoke('list_network_impacted_customers', { token: getTokenOrThrow(), ...(params || {}) }),
  },
};
