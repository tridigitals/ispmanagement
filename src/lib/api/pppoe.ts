import { getTokenOrThrow, safeInvoke } from './core';
import type { PaginatedResponse, PppoeAccountPublic } from './types';

export const pppoe = {
  accounts: {
    list: (params?: {
      customer_id?: string;
      location_id?: string;
      router_id?: string;
      q?: string;
      page?: number;
      per_page?: number;
    }): Promise<PaginatedResponse<PppoeAccountPublic>> =>
      safeInvoke('list_pppoe_accounts', { token: getTokenOrThrow(), ...(params || {}) }),

    get: (id: string): Promise<PppoeAccountPublic> =>
      safeInvoke('get_pppoe_account', { token: getTokenOrThrow(), id }),

    create: (dto: {
      router_id: string;
      customer_id: string;
      location_id: string;
      username: string;
      password: string;
      package_id?: string | null;
      profile_id?: string | null;
      router_profile_name?: string | null;
      remote_address?: string | null;
      address_pool?: string | null;
      disabled?: boolean;
      comment?: string | null;
    }): Promise<PppoeAccountPublic> =>
      safeInvoke('create_pppoe_account', {
        token: getTokenOrThrow(),
        router_id: dto.router_id,
        customer_id: dto.customer_id,
        location_id: dto.location_id,
        username: dto.username,
        password: dto.password,
        package_id: dto.package_id ?? null,
        profile_id: dto.profile_id ?? null,
        router_profile_name: dto.router_profile_name ?? null,
        remote_address: dto.remote_address ?? null,
        address_pool: dto.address_pool ?? null,
        disabled: dto.disabled ?? false,
        comment: dto.comment ?? null,
      }),

    update: (
      id: string,
      dto: {
        username?: string;
        password?: string;
        package_id?: string | null;
        profile_id?: string | null;
        router_profile_name?: string | null;
        remote_address?: string | null;
        address_pool?: string | null;
        disabled?: boolean;
        comment?: string | null;
      },
    ): Promise<PppoeAccountPublic> =>
      safeInvoke('update_pppoe_account', {
        token: getTokenOrThrow(),
        id,
        username: dto.username,
        password: dto.password,
        package_id: dto.package_id ?? undefined,
        profile_id: dto.profile_id ?? undefined,
        router_profile_name: dto.router_profile_name ?? undefined,
        remote_address: dto.remote_address ?? undefined,
        address_pool: dto.address_pool ?? undefined,
        disabled: dto.disabled,
        comment: dto.comment ?? undefined,
      }),

    delete: (id: string): Promise<void> =>
      safeInvoke('delete_pppoe_account', { token: getTokenOrThrow(), id }),

    apply: (id: string): Promise<PppoeAccountPublic> =>
      safeInvoke('apply_pppoe_account', { token: getTokenOrThrow(), id }),

    reconcileRouter: (routerId: string): Promise<any> =>
      safeInvoke('reconcile_pppoe_router', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
  },

  import: {
    preview: (routerId: string, params?: { include_disabled?: boolean }): Promise<any[]> =>
      safeInvoke('preview_pppoe_import_from_router', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
        include_disabled: params?.include_disabled,
        includeDisabled: params?.include_disabled,
      }),

    run: (
      routerId: string,
      dto: { usernames: string[]; customer_id?: string; location_id?: string },
    ): Promise<any> =>
      safeInvoke('import_pppoe_from_router', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
        usernames: dto.usernames,
        customer_id: dto.customer_id,
        location_id: dto.location_id,
      }),
  },
};
