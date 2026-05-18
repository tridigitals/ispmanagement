import type { NetworkAsset } from '$lib/api/client';
import type { NMLink, NMNode, NMRouter } from '$lib/components/network/networkMapUtils';

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
const ASSET_PORT_RANK: Record<string, number> = {
  olt: 0,
  odf: 1,
  switch: 2,
  odc: 3,
  splitter: 4,
  fat: 5,
  nap: 6,
  odp: 7,
  ont: 8,
  onu: 8,
  media_converter: 8,
};
export type NetworkAssetOccupancyTopologyArgs = {
  assetNodeIdsByAssetId?: Map<string, string>;
  nodeRows?: Pick<NMNode, 'id' | 'node_type' | 'status' | 'metadata'>[];
  linkRows?: Pick<NMLink, 'from_node_id' | 'to_node_id'>[];
  routerRows?: Pick<NMRouter, 'id' | 'name' | 'identity'>[];
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
  const cached = readCachedPortOccupancy(asset.metadata);
  if (cached) return cached;
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
      let otherNodeId = '';
      if (fromNodeId === sourceNodeId) otherNodeId = toNodeId;
      else if (toNodeId === sourceNodeId) otherNodeId = fromNodeId;
      if (!otherNodeId) continue;

      const endpointKey = buildOccupancyEndpointKeyFromTopologyLink({
        sourceAssetType: asset.asset_type,
        otherNodeId,
        nodeRows: topology?.nodeRows || [],
        routerRows: topology?.routerRows || [],
      });
      if (endpointKey) endpointKeys.add(endpointKey);
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

function readCachedPortOccupancy(metadata: NetworkAsset['metadata'] | undefined): NetworkAssetPortOccupancy | null {
  const total = parsePositiveInteger(metadata?.port_usage_total);
  const used = parseNonNegativeInteger(metadata?.port_usage_used);
  const available = parseNonNegativeInteger(metadata?.port_usage_available);
  const state = String(metadata?.port_usage_state || '').trim();
  if (total == null || used == null || available == null) return null;
  if (state !== 'empty' && state !== 'partial' && state !== 'full') return null;
  return { total, used, available, state };
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

function parseNonNegativeInteger(value: unknown): number | null {
  if (typeof value === 'number' && Number.isInteger(value) && value >= 0) {
    return value;
  }
  if (typeof value === 'string' && /^\d+$/.test(value.trim())) {
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

function buildOccupancyEndpointKeyFromTopologyLink(args: {
  sourceAssetType: string;
  otherNodeId: string;
  nodeRows: Pick<NMNode, 'id' | 'node_type' | 'status' | 'metadata'>[];
  routerRows: Pick<NMRouter, 'id'>[];
}): string | null {
  const normalizedNodeId = String(args.otherNodeId || '').trim();
  if ((args.routerRows || []).some((router) => String(router.id || '').trim() === normalizedNodeId)) {
    return `router:${normalizedNodeId}`;
  }

  const otherNode = (args.nodeRows || []).find((node) => node.id === normalizedNodeId);
  if (!otherNode) return null;

  const source = String(otherNode.metadata?.asset_source || otherNode.metadata?.asset_type || '').trim();
  if (source === 'network_asset') {
    const assetId = String(otherNode.metadata?.asset_id || '').trim();
    const targetAssetType = String(otherNode.metadata?.asset_type || '').trim().toLowerCase();
    if (!assetId || !shouldCountTopologyAssetPortUsage(args.sourceAssetType, targetAssetType)) {
      return null;
    }
    return `asset:${assetId}`;
  }

  if (!isLinkedCustomerLocationNode(otherNode)) return null;
  return buildOccupancyEndpointKeyFromNode(otherNode);
}

function shouldCountTopologyAssetPortUsage(sourceAssetType: string, targetAssetType: string): boolean {
  const sourceRank = ASSET_PORT_RANK[String(sourceAssetType || '').trim().toLowerCase()];
  const targetRank = ASSET_PORT_RANK[String(targetAssetType || '').trim().toLowerCase()];
  if (!Number.isFinite(sourceRank) || !Number.isFinite(targetRank)) return false;
  return targetRank >= sourceRank;
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
