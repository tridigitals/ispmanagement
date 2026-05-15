import { getTokenOrThrow, safeInvoke } from './core';
import type {
  CreateNetworkAssetRequest,
  NetworkAsset,
  NetworkAssetListItem,
  PaginatedResponse,
  UpdateNetworkAssetRequest,
} from './types';

export const networkAssets = {
  list: (
    params?: {
      q?: string;
      asset_type?: string;
      status?: string;
      customer_id?: string;
      location_id?: string;
      parent_asset_id?: string;
      page?: number;
      per_page?: number;
    },
  ): Promise<PaginatedResponse<NetworkAssetListItem>> =>
    safeInvoke('list_network_assets', { token: getTokenOrThrow(), ...(params || {}) }),

  get: (id: string): Promise<NetworkAsset> =>
    safeInvoke('get_network_asset', { token: getTokenOrThrow(), id }),

  create: (dto: CreateNetworkAssetRequest): Promise<NetworkAsset> =>
    safeInvoke('create_network_asset', { token: getTokenOrThrow(), ...dto }),

  update: (id: string, dto: UpdateNetworkAssetRequest): Promise<NetworkAsset> =>
    safeInvoke('update_network_asset', { token: getTokenOrThrow(), id, ...dto }),

  delete: (id: string): Promise<void> =>
    safeInvoke('delete_network_asset', { token: getTokenOrThrow(), id }),

  assignCustomer: (id: string, customer_id: string | null): Promise<NetworkAsset> =>
    safeInvoke('assign_network_asset_customer', {
      token: getTokenOrThrow(),
      id,
      customer_id,
    }),

  assignLocation: (id: string, location_id: string | null): Promise<NetworkAsset> =>
    safeInvoke('assign_network_asset_location', {
      token: getTokenOrThrow(),
      id,
      location_id,
    }),

  assignWorkOrder: (id: string, work_order_id: string | null): Promise<NetworkAsset> =>
    safeInvoke('assign_network_asset_work_order', {
      token: getTokenOrThrow(),
      id,
      work_order_id,
    }),

  linkParentAsset: (id: string, parent_asset_id: string | null): Promise<NetworkAsset> =>
    safeInvoke('link_network_asset_parent', {
      token: getTokenOrThrow(),
      id,
      parent_asset_id,
    }),

  listCustomerAssets: (customer_id: string): Promise<NetworkAssetListItem[]> =>
    safeInvoke('list_customer_network_assets', {
      token: getTokenOrThrow(),
      customer_id,
    }),
};
