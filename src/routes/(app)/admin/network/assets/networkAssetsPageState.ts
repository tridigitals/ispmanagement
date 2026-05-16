import type {
  CreateNetworkAssetRequest,
  NetworkAssetListItem,
} from '$lib/api/client';
import { buildNetworkAssetOccupancyLabel } from '$lib/utils/networkAssetOccupancy';

export type SelectOption = {
  value: string;
  label: string;
};

export function filterSelectOptions(options: SelectOption[], query?: string): SelectOption[] {
  const normalizedQuery = (query || '').trim().toLowerCase();
  if (!normalizedQuery) {
    return options;
  }

  return options.filter((option) => option.label.toLowerCase().includes(normalizedQuery));
}

export function normalizeNetworkAssetSearch(value: string): string {
  return value.trim().toLowerCase();
}

export function filterNetworkAssets(
  assets: NetworkAssetListItem[],
  filters: {
    q?: string;
    assetType?: string;
    status?: string;
  },
): NetworkAssetListItem[] {
  const q = normalizeNetworkAssetSearch(filters.q || '');
  return [...assets]
    .filter((asset) => {
      if (filters.assetType && filters.assetType !== 'all' && asset.asset_type !== filters.assetType) {
        return false;
      }
      if (filters.status && filters.status !== 'all' && asset.status !== filters.status) {
        return false;
      }
      if (!q) return true;
      return [asset.name, asset.code || '', asset.serial_number || '']
        .join(' ')
        .toLowerCase()
        .includes(q);
    })
    .sort((a, b) => String(b.updated_at).localeCompare(String(a.updated_at)));
}

export function buildNetworkAssetStats(assets: NetworkAssetListItem[]) {
  return {
    total: assets.length,
    installed: assets.filter((asset) => asset.status === 'installed').length,
    available: assets.filter((asset) => asset.status === 'available').length,
    faulty: assets.filter((asset) => asset.status === 'faulty').length,
  };
}

const DISTRIBUTION_ASSET_TYPES = new Set(['olt', 'odc', 'odp', 'fat', 'nap', 'switch']);

export function isDistributionAssetType(assetType: string): boolean {
  return DISTRIBUTION_ASSET_TYPES.has(String(assetType || '').trim());
}

export function buildNetworkAssetRelationText(
  asset: Pick<NetworkAssetListItem, 'asset_type' | 'customer_name' | 'parent_asset_name'>,
): string {
  if (isDistributionAssetType(asset.asset_type)) {
    const upstream = String(asset.parent_asset_name || '').trim();
    return upstream ? `Upstream: ${upstream}` : 'Topology-managed';
  }

  return String(asset.customer_name || '').trim() || '—';
}

export function buildNetworkAssetTopologyText(
  asset: Pick<
    NetworkAssetListItem,
    'asset_type' | 'location_label' | 'parent_asset_name' | 'id' | 'metadata'
  >,
  allAssets: Pick<
    NetworkAssetListItem,
    'id' | 'asset_type' | 'parent_asset_id' | 'status' | 'customer_id' | 'location_id' | 'metadata'
  >[],
): string {
  if (isDistributionAssetType(asset.asset_type)) {
    const occupancy = buildNetworkAssetOccupancyLabel(asset, allAssets);
    if (occupancy) return occupancy;
    const upstream = String(asset.parent_asset_name || '').trim();
    return upstream ? 'Linked upstream' : 'No topology link yet';
  }

  return String(asset.location_label || '').trim() || '—';
}

export function buildNetworkAssetSavePayload(args: {
  draft: {
    asset_type: string;
    name: string;
    code: string;
    vendor: string;
    model: string;
    serial_number: string;
    status: string;
    latitude: string;
    longitude: string;
    notes: string;
  };
  metadata: Record<string, unknown> | null;
  existingRelations?: {
    customer_id?: string | null;
    location_id?: string | null;
    work_order_id?: string | null;
    parent_asset_id?: string | null;
  };
}): CreateNetworkAssetRequest {
  const latitude = args.draft.latitude.trim() ? Number(args.draft.latitude) : null;
  const longitude = args.draft.longitude.trim() ? Number(args.draft.longitude) : null;

  return {
    asset_type: args.draft.asset_type,
    name: args.draft.name,
    code: args.draft.code || null,
    vendor: args.draft.vendor || null,
    model: args.draft.model || null,
    serial_number: args.draft.serial_number || null,
    status: args.draft.status,
    customer_id: args.existingRelations?.customer_id ?? null,
    location_id: args.existingRelations?.location_id ?? null,
    work_order_id: args.existingRelations?.work_order_id ?? null,
    parent_asset_id: args.existingRelations?.parent_asset_id ?? null,
    latitude,
    longitude,
    notes: args.draft.notes || null,
    metadata: args.metadata,
  };
}
