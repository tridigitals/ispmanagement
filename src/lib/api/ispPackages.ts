import { getTokenOrThrow, safeInvoke } from './core';
import type { IspPackage, IspPackageRouterMappingView, PaginatedResponse } from './types';

export const ispPackages = {
  packages: {
    list: (params?: {
      q?: string;
      page?: number;
      per_page?: number;
      sort_by?: string;
      sort_dir?: 'asc' | 'desc';
    }): Promise<PaginatedResponse<IspPackage>> =>
      safeInvoke('list_isp_packages', { token: getTokenOrThrow(), ...(params || {}) }),

    create: (dto: {
      service_type?: string;
      name: string;
      description?: string | null;
      features?: string[];
      is_active?: boolean;
      price_monthly?: number;
      price_yearly?: number;
    }): Promise<IspPackage> =>
      safeInvoke('create_isp_package', {
        token: getTokenOrThrow(),
        service_type: dto.service_type ?? 'internet_pppoe',
        name: dto.name,
        description: dto.description ?? null,
        features: dto.features ?? [],
        is_active: dto.is_active ?? true,
        price_monthly: dto.price_monthly ?? 0,
        price_yearly: dto.price_yearly ?? 0,
      }),

    update: (
      id: string,
      dto: {
        service_type?: string;
        name?: string;
        description?: string | null;
        features?: string[];
        is_active?: boolean;
        price_monthly?: number;
        price_yearly?: number;
      },
    ): Promise<IspPackage> =>
      safeInvoke('update_isp_package', {
        token: getTokenOrThrow(),
        id,
        service_type: dto.service_type ?? undefined,
        name: dto.name,
        description: dto.description ?? undefined,
        features: dto.features,
        is_active: dto.is_active,
        price_monthly: dto.price_monthly,
        price_yearly: dto.price_yearly,
      }),

    delete: (id: string): Promise<void> =>
      safeInvoke('delete_isp_package', { token: getTokenOrThrow(), id }),
  },

  routerMappings: {
    list: (params?: { router_id?: string }): Promise<IspPackageRouterMappingView[]> =>
      safeInvoke('list_isp_package_router_mappings', {
        token: getTokenOrThrow(),
        router_id: params?.router_id,
      }),

    upsert: (dto: {
      router_id: string;
      package_id: string;
      router_profile_name: string;
      address_pool?: string | null;
    }): Promise<any> =>
      safeInvoke('upsert_isp_package_router_mapping', {
        token: getTokenOrThrow(),
        router_id: dto.router_id,
        package_id: dto.package_id,
        router_profile_name: dto.router_profile_name,
        address_pool: dto.address_pool ?? null,
      }),
  },
};
