import type { NetworkAssetListItem } from '$lib/api/types';
import type { NMNode } from './networkMapUtils';

export type TopologyAssetConnectDraft = {
  parentAssetId: string;
  customerId: string;
  locationId: string;
};

export type SelectOption = {
  value: string;
  label: string;
};

export type TopologyAssetConnectionOperation = {
  assetId: string;
  parentAssetId?: string | null;
  customerId?: string | null;
  locationId?: string | null;
};

export type ResolveTopologyAssetNodeIdArgs = {
  assetId: string;
  assetType: string;
  latitude?: number | null;
  longitude?: number | null;
  nodeRows: Pick<NMNode, 'id' | 'metadata'>[];
  cachedNodeId?: string | null;
  syncNodes: () => Promise<void>;
  refreshNodeRows: () => Promise<Pick<NMNode, 'id' | 'metadata'>[]>;
  fetchNearbyNodeRows: (args: {
    assetType: string;
    latitude: number;
    longitude: number;
  }) => Promise<Pick<NMNode, 'id' | 'metadata'>[]>;
};

const CUSTOMER_DROP_TYPES = new Set(['odp', 'fat', 'nap']);
const UPSTREAM_RANK: Record<string, number> = {
  olt: 0,
  odf: 1,
  switch: 2,
  odc: 3,
  splitter: 4,
  fat: 5,
  nap: 6,
  odp: 7,
};
const PREFERRED_PARENT_TYPES: Record<string, string[]> = {
  odf: ['olt', 'switch'],
  odc: ['odf', 'olt', 'switch'],
  splitter: ['odc', 'odf', 'switch', 'olt'],
  fat: ['splitter', 'odc', 'odf', 'switch', 'olt'],
  nap: ['fat', 'splitter', 'odc', 'odf', 'switch', 'olt'],
  odp: ['nap', 'fat', 'splitter', 'odc', 'odf', 'switch', 'olt'],
};
const TOPOLOGY_ASSET_NODE_SOURCE = 'network_asset';

export function canTopologyAssetAcceptConnection(args: {
  assetType: string;
  portCapacity?: number | null;
  portsAvailable?: number | null;
}) {
  if (String(args.assetType || '').trim() !== 'odp') return true;
  if (!Number.isFinite(args.portCapacity) || Number(args.portCapacity) <= 0) return true;
  return Number(args.portsAvailable ?? 0) > 0;
}

export function assetSupportsCustomerDrop(assetType: string): boolean {
  return CUSTOMER_DROP_TYPES.has(String(assetType || '').trim());
}

export function assetSupportsUpstreamLink(assetType: string): boolean {
  const normalized = String(assetType || '').trim();
  return normalized in UPSTREAM_RANK && normalized !== 'olt';
}

export function buildTopologyAssetConnectDraft(
  asset: Pick<NetworkAssetListItem, 'parent_asset_id' | 'customer_id' | 'location_id'>,
): TopologyAssetConnectDraft {
  return {
    parentAssetId: String(asset.parent_asset_id || '').trim(),
    customerId: String(asset.customer_id || '').trim(),
    locationId: String(asset.location_id || '').trim(),
  };
}

export function buildTopologyAssetParentOptions(args: {
  assetId: string;
  assetType: string;
  currentParentAssetId?: string | null;
  assets: NetworkAssetListItem[];
}): SelectOption[] {
  const currentTypeRank = UPSTREAM_RANK[String(args.assetType || '').trim()];
  const currentParentAssetId = String(args.currentParentAssetId || '').trim();

  const eligible = (args.assets || []).filter((asset) => {
    if (asset.id === args.assetId) return false;
    const candidateType = String(asset.asset_type || '').trim();
    const candidateRank = UPSTREAM_RANK[candidateType];
    if (!Number.isFinite(currentTypeRank) || !Number.isFinite(candidateRank)) {
      return asset.id === currentParentAssetId;
    }
    if (asset.id === currentParentAssetId) return true;
    return candidateRank < currentTypeRank;
  });

  return eligible
    .map((asset) => ({
      value: asset.id,
      label: `${asset.name} (${String(asset.asset_type || '').toUpperCase()})`,
      sortScore: assetParentSortScore(args.assetType, asset.asset_type, asset.id === currentParentAssetId),
    }))
    .sort((a, b) => a.sortScore - b.sortScore || a.label.localeCompare(b.label))
    .map(({ value, label }) => ({ value, label }));
}

export function findTopologyAssetNodeId(
  nodeRows: Pick<NMNode, 'id' | 'metadata'>[],
  assetId: string,
): string {
  const normalizedAssetId = String(assetId || '').trim();
  if (!normalizedAssetId) return '';
  const matched = (nodeRows || []).find((node) => {
    const assetSource = String(node.metadata?.asset_source || node.metadata?.asset_type || '').trim();
    const candidateAssetId = String(node.metadata?.asset_id || '').trim();
    return assetSource === TOPOLOGY_ASSET_NODE_SOURCE && candidateAssetId === normalizedAssetId;
  });
  return matched?.id || '';
}

export async function resolveTopologyAssetNodeId(
  args: ResolveTopologyAssetNodeIdArgs,
): Promise<string> {
  const cachedNodeId = String(args.cachedNodeId || '').trim();
  if (cachedNodeId) return cachedNodeId;

  let nodeId = findTopologyAssetNodeId(args.nodeRows, args.assetId);
  if (nodeId) return nodeId;

  await args.syncNodes();
  nodeId = findTopologyAssetNodeId(await args.refreshNodeRows(), args.assetId);
  if (nodeId) return nodeId;

  if (!Number.isFinite(args.latitude) || !Number.isFinite(args.longitude)) return '';
  nodeId = findTopologyAssetNodeId(
    await args.fetchNearbyNodeRows({
      assetType: args.assetType,
      latitude: Number(args.latitude),
      longitude: Number(args.longitude),
    }),
    args.assetId,
  );
  return nodeId;
}

export function buildTopologyAssetConnectionOperations(args: {
  sourceAsset: Pick<
    NetworkAssetListItem,
    'id' | 'asset_type' | 'parent_asset_id' | 'customer_id' | 'location_id'
  >;
  targetNode: Pick<NMNode, 'metadata' | 'node_type'> | null | undefined;
}): TopologyAssetConnectionOperation[] {
  const targetNode = args.targetNode;
  if (!targetNode) return [];

  const sourceAssetType = String(args.sourceAsset.asset_type || '').trim();
  const targetAssetSource = String(
    targetNode.metadata?.asset_source || targetNode.metadata?.asset_type || '',
  ).trim();
  const targetAssetType = String(targetNode.metadata?.asset_type || targetNode.node_type || '').trim();
  const targetAssetId = String(targetNode.metadata?.asset_id || '').trim();

  if (targetAssetSource === 'customer_location' && assetSupportsCustomerDrop(sourceAssetType)) {
    return [];
  }

  if (targetAssetSource !== TOPOLOGY_ASSET_NODE_SOURCE || !targetAssetId) return [];

  const sourceRank = UPSTREAM_RANK[sourceAssetType];
  const targetRank = UPSTREAM_RANK[targetAssetType];
  if (!Number.isFinite(sourceRank) || !Number.isFinite(targetRank)) return [];

  if (assetSupportsUpstreamLink(sourceAssetType) && targetRank < sourceRank) {
    return [
      {
        assetId: args.sourceAsset.id,
        parentAssetId: targetAssetId,
      },
    ];
  }

  if (assetSupportsUpstreamLink(targetAssetType) && sourceRank < targetRank) {
    return [
      {
        assetId: targetAssetId,
        parentAssetId: args.sourceAsset.id,
      },
    ];
  }

  return [];
}

function assetParentSortScore(assetType: string, parentType: string, isCurrentParent: boolean): number {
  if (isCurrentParent) return -1;
  const preferred = PREFERRED_PARENT_TYPES[String(assetType || '').trim()] || [];
  const index = preferred.indexOf(String(parentType || '').trim());
  if (index >= 0) return index;
  return 999;
}
