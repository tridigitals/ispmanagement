import type { NetworkAsset } from '$lib/api/client';

type OccupancyState = 'empty' | 'partial' | 'full';

export type NetworkAssetPortOccupancy = {
  total: number;
  used: number;
  available: number;
  state: OccupancyState;
};

const TERMINAL_TYPES = new Set(['ont', 'onu']);
const EXCLUDED_STATUSES = new Set(['faulty', 'retired']);
const DIRECT_ATTACHMENT_TYPES = new Set(['media_converter', 'ont', 'onu']);

export function getNetworkAssetPortOccupancy(
  asset: Pick<NetworkAsset, 'id' | 'asset_type' | 'metadata'>,
  allAssets: Pick<
    NetworkAsset,
    'id' | 'asset_type' | 'parent_asset_id' | 'status' | 'customer_id' | 'location_id'
  >[],
): NetworkAssetPortOccupancy | null {
  if (asset.asset_type !== 'odp') return null;
  const total = parsePositiveInteger(asset.metadata?.total_port_capacity);
  if (!total) return null;

  const endpointKeys = new Set<string>();
  for (const item of allAssets) {
    if (item.parent_asset_id !== asset.id) continue;
    if (EXCLUDED_STATUSES.has(item.status)) continue;
    if (!TERMINAL_TYPES.has(item.asset_type) && !DIRECT_ATTACHMENT_TYPES.has(item.asset_type)) {
      continue;
    }
    endpointKeys.add(buildOccupancyEndpointKey(item));
  }

  const used = endpointKeys.size;
  const available = Math.max(total - used, 0);

  return {
    total,
    used,
    available,
    state: used <= 0 ? 'empty' : available <= 0 ? 'full' : 'partial',
  };
}

export function getNetworkAssetPortOccupancySummary(
  asset: Pick<NetworkAsset, 'id' | 'asset_type' | 'metadata'>,
  allAssets: Pick<
    NetworkAsset,
    'id' | 'asset_type' | 'parent_asset_id' | 'status' | 'customer_id' | 'location_id'
  >[],
): string[] {
  const occupancy = getNetworkAssetPortOccupancy(asset, allAssets);
  if (!occupancy) return [];

  return [
    `Port Capacity: ${occupancy.total}`,
    `Ports Used: ${occupancy.used}`,
    `Ports Available: ${occupancy.available}`,
  ];
}

export function buildNetworkAssetOccupancyLabel(
  asset: Pick<NetworkAsset, 'id' | 'asset_type' | 'metadata'>,
  allAssets: Pick<
    NetworkAsset,
    'id' | 'asset_type' | 'parent_asset_id' | 'status' | 'customer_id' | 'location_id'
  >[],
): string | null {
  const occupancy = getNetworkAssetPortOccupancy(asset, allAssets);
  if (!occupancy) return null;
  return `${occupancy.used}/${occupancy.total} used`;
}

function parsePositiveInteger(value: unknown): number | null {
  if (typeof value === 'number' && Number.isInteger(value) && value > 0) {
    return value;
  }
  if (typeof value === 'string' && /^[1-9]\d*$/.test(value.trim())) {
    return Number(value.trim());
  }
  return null;
}

function buildOccupancyEndpointKey(
  asset: Pick<NetworkAsset, 'id' | 'customer_id' | 'location_id'>,
): string {
  const locationId = String(asset.location_id || '').trim();
  if (locationId) return `location:${locationId}`;
  const customerId = String(asset.customer_id || '').trim();
  if (customerId) return `customer:${customerId}`;
  return `asset:${asset.id}`;
}
