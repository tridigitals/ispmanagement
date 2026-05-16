import type { NetworkAsset } from '$lib/api/client';
import type { NMLink, NMNode } from '$lib/components/network/networkMapUtils';

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
export type NetworkAssetOccupancyTopologyArgs = {
  assetNodeIdsByAssetId?: Map<string, string>;
  nodeRows?: Pick<NMNode, 'id' | 'node_type' | 'status' | 'metadata'>[];
  linkRows?: Pick<NMLink, 'from_node_id' | 'to_node_id'>[];
};

export function getNetworkAssetPortOccupancy(
  asset: Pick<NetworkAsset, 'id' | 'asset_type' | 'metadata'>,
  allAssets: Pick<
    NetworkAsset,
    'id' | 'asset_type' | 'parent_asset_id' | 'status' | 'customer_id' | 'location_id'
  >[],
  topology?: NetworkAssetOccupancyTopologyArgs,
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

  const sourceNodeId =
    topology?.assetNodeIdsByAssetId?.get(asset.id) ||
    resolveTopologyAssetNodeId(topology?.nodeRows || [], asset.id);
  if (sourceNodeId) {
    for (const row of topology?.linkRows || []) {
      const fromNodeId = String(row.from_node_id || '').trim();
      const toNodeId = String(row.to_node_id || '').trim();
      let customerNodeId = '';
      if (fromNodeId === sourceNodeId) customerNodeId = toNodeId;
      else if (toNodeId === sourceNodeId) customerNodeId = fromNodeId;
      if (!customerNodeId) continue;

      const customerNode = (topology?.nodeRows || []).find((node) => node.id === customerNodeId);
      if (!isLinkedCustomerLocationNode(customerNode)) continue;
      endpointKeys.add(buildOccupancyEndpointKeyFromNode(customerNode));
    }
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
  topology?: NetworkAssetOccupancyTopologyArgs,
): string[] {
  const occupancy = getNetworkAssetPortOccupancy(asset, allAssets, topology);
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
  topology?: NetworkAssetOccupancyTopologyArgs,
): string | null {
  const occupancy = getNetworkAssetPortOccupancy(asset, allAssets, topology);
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

function buildOccupancyEndpointKeyFromNode(
  node: Pick<NMNode, 'id' | 'metadata'>,
): string {
  const locationId = String(node.metadata?.location_id || '').trim();
  if (locationId) return `location:${locationId}`;
  const customerId = String(node.metadata?.customer_id || '').trim();
  if (customerId) return `customer:${customerId}`;
  return `node:${node.id}`;
}

function resolveTopologyAssetNodeId(
  nodeRows: Pick<NMNode, 'id' | 'metadata'>[],
  assetId: string,
): string {
  const normalizedAssetId = String(assetId || '').trim();
  if (!normalizedAssetId) return '';
  return (
    nodeRows.find((node) => {
      const source = String(node.metadata?.asset_source || node.metadata?.asset_type || '').trim();
      const candidateAssetId = String(node.metadata?.asset_id || '').trim();
      return source === 'network_asset' && candidateAssetId === normalizedAssetId;
    })?.id || ''
  );
}

function isLinkedCustomerLocationNode(
  node: Pick<NMNode, 'id' | 'node_type' | 'status' | 'metadata'> | undefined,
): node is Pick<NMNode, 'id' | 'node_type' | 'status' | 'metadata'> {
  if (!node) return false;
  const source = String(node.metadata?.asset_source || node.metadata?.asset_type || '').trim();
  if (source !== 'customer_location') return false;
  return String(node.node_type || '').trim() === 'customer_premise';
}
