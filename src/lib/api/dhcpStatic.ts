import { getTokenOrThrow, safeInvoke } from './core';
import type { DhcpStaticServicePublic, PaginatedResponse } from './types';

export const dhcpStatic = {
  services: {
    list: (params?: {
      customer_id?: string;
      location_id?: string;
      router_id?: string;
      dhcp_server_name?: string;
      q?: string;
      page?: number;
      per_page?: number;
    }): Promise<PaginatedResponse<DhcpStaticServicePublic>> =>
      safeInvoke('list_dhcp_static_services', { token: getTokenOrThrow(), ...(params || {}) }),

    get: (id: string): Promise<DhcpStaticServicePublic> =>
      safeInvoke('get_dhcp_static_service', { token: getTokenOrThrow(), id }),

    create: (dto: {
      subscription_id: string;
      router_id: string;
      customer_id: string;
      location_id: string;
      package_id: string;
      dhcp_server_name: string;
      mac_address: string;
      ip_address: string;
      comment?: string | null;
      disabled?: boolean;
      queue_mode?: 'none' | 'simple_queue' | null;
      queue_rate_limit?: string | null;
      work_order_id?: string | null;
    }): Promise<DhcpStaticServicePublic> =>
      safeInvoke('create_dhcp_static_service', {
        token: getTokenOrThrow(),
        ...dto,
        comment: dto.comment ?? null,
        disabled: dto.disabled ?? false,
        queue_mode: dto.queue_mode ?? 'none',
        queue_rate_limit: dto.queue_rate_limit ?? null,
        work_order_id: dto.work_order_id ?? null,
      }),

    update: (
      id: string,
      dto: {
        router_id?: string;
        package_id?: string;
        dhcp_server_name?: string;
        mac_address?: string;
        ip_address?: string;
        comment?: string | null;
        disabled?: boolean;
        queue_mode?: 'none' | 'simple_queue' | null;
        queue_rate_limit?: string | null;
        work_order_id?: string | null;
      },
    ): Promise<DhcpStaticServicePublic> =>
      safeInvoke('update_dhcp_static_service', {
        token: getTokenOrThrow(),
        id,
        ...dto,
        work_order_id: dto.work_order_id ?? undefined,
      }),

    delete: (id: string): Promise<void> =>
      safeInvoke('delete_dhcp_static_service', { token: getTokenOrThrow(), id }),

    apply: (id: string, params?: { work_order_id?: string | null }): Promise<DhcpStaticServicePublic> =>
      safeInvoke('apply_dhcp_static_service', {
        token: getTokenOrThrow(),
        id,
        work_order_id: params?.work_order_id ?? undefined,
      }),

    reconcileRouter: (routerId: string): Promise<any> =>
      safeInvoke('reconcile_dhcp_static_router', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
  },
};
