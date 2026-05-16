import type { FeatureCollection, Point } from 'geojson';

import type { NetworkAssetListItem } from '$lib/api/types';
import {
  getNetworkAssetPortOccupancy,
  type NetworkAssetOccupancyTopologyArgs,
} from '$lib/utils/networkAssetOccupancy';
import { getNetworkAssetTypeLabel } from '$lib/utils/networkAssetTypes';
import { isCustomerNodeType, type NMLink, type NMNode } from './networkMapUtils';

const TOPOLOGY_ASSET_TYPES = ['olt', 'odf', 'odc', 'splitter', 'fat', 'nap', 'odp', 'switch'] as const;
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

export type TopologyAssetType = (typeof TOPOLOGY_ASSET_TYPES)[number];

export type TopologyAssetMarkerSpec = {
  assetType: string;
  label: string;
  legendLabel: string;
  color: string;
};

export type TopologyAssetRow = {
  id: string;
  name: string;
  assetType: string;
  assetTypeLabel: string;
  status: string;
  code: string | null;
  serialNumber: string | null;
  customerName: string | null;
  locationLabel: string | null;
  latitude: number;
  longitude: number;
  customerId: string | null;
  locationId: string | null;
  parentAssetId: string | null;
  markerLabel: string;
  markerColor: string;
  legendLabel: string;
  portCapacity: number | null;
  portsUsed: number | null;
  portsAvailable: number | null;
  hasUpstreamRelation: boolean;
  hasCustomerRelation: boolean;
};

export function isTopologyAssetType(assetType: string): assetType is TopologyAssetType {
  return TOPOLOGY_ASSET_TYPES.includes(assetType as TopologyAssetType);
}

export function getTopologyAssetMarkerSpec(assetType: string): TopologyAssetMarkerSpec {
  if (assetType === 'olt') {
    return { assetType, label: 'OLT', legendLabel: 'OLT', color: '#b45309' };
  }
  if (assetType === 'odf') {
    return { assetType, label: 'ODF', legendLabel: 'ODF', color: '#1d4ed8' };
  }
  if (assetType === 'odc') {
    return { assetType, label: 'ODC', legendLabel: 'ODC', color: '#2563eb' };
  }
  if (assetType === 'splitter') {
    return { assetType, label: 'SPL', legendLabel: 'Splitter', color: '#7c3aed' };
  }
  if (assetType === 'odp') {
    return { assetType, label: 'ODP', legendLabel: 'ODP', color: '#0f766e' };
  }
  if (assetType === 'fat') {
    return { assetType, label: 'FAT', legendLabel: 'FAT', color: '#c2410c' };
  }
  if (assetType === 'nap') {
    return { assetType, label: 'NAP', legendLabel: 'NAP', color: '#be123c' };
  }
  if (assetType === 'switch') {
    return { assetType, label: 'SW', legendLabel: 'Switch', color: '#475569' };
  }
  return {
    assetType,
    label: 'AS',
    legendLabel: getNetworkAssetTypeLabel(assetType),
    color: '#64748b',
  };
}

export function buildTopologyAssetRows(
  assets: NetworkAssetListItem[],
  topology?: NetworkAssetOccupancyTopologyArgs,
): TopologyAssetRow[] {
  return (assets || [])
    .filter((asset) => isTopologyAssetType(String(asset.asset_type || '').trim()))
    .filter(
      (asset) => Number.isFinite(asset.latitude) && Number.isFinite(asset.longitude),
    )
    .map((asset) => {
      const assetType = String(asset.asset_type || '').trim();
      const marker = getTopologyAssetMarkerSpec(assetType);
      const occupancy = getNetworkAssetPortOccupancy(asset, assets, topology);
      const hasUpstreamRelation = hasTopologyAssetUpstreamRelation(asset, assets, topology);
      const hasCustomerRelation = hasTopologyAssetCustomerRelation(asset, assets, topology);
      return {
        id: asset.id,
        name: asset.name,
        assetType,
        assetTypeLabel: getNetworkAssetTypeLabel(assetType),
        status: String(asset.status || ''),
        code: asset.code,
        serialNumber: asset.serial_number,
        customerName: asset.customer_name,
        locationLabel: asset.location_label,
        latitude: Number(asset.latitude),
        longitude: Number(asset.longitude),
        customerId: asset.customer_id,
        locationId: asset.location_id,
        parentAssetId: asset.parent_asset_id,
        markerLabel: marker.label,
        markerColor: marker.color,
        legendLabel: marker.legendLabel,
        portCapacity: occupancy?.total ?? null,
        portsUsed: occupancy?.used ?? null,
        portsAvailable: occupancy?.available ?? null,
        hasUpstreamRelation,
        hasCustomerRelation,
      };
    });
}

export function topologyAssetsToFeatureCollection(
  rows: TopologyAssetRow[],
): FeatureCollection<Point, Record<string, string | number | null>> {
  return {
    type: 'FeatureCollection',
    features: (rows || []).map((row) => ({
      type: 'Feature',
      geometry: {
        type: 'Point',
        coordinates: [row.longitude, row.latitude],
      },
      properties: {
        id: row.id,
        name: row.name,
        asset_type: row.assetType,
        asset_type_label: row.assetTypeLabel,
        customer_id: row.customerId,
        location_id: row.locationId,
        parent_asset_id: row.parentAssetId,
        marker_label: row.markerLabel,
        marker_color: row.markerColor,
        status: row.status,
        code: row.code,
        serial_number: row.serialNumber,
        customer_name: row.customerName,
        location_label: row.locationLabel,
        port_capacity: row.portCapacity,
        ports_used: row.portsUsed,
        ports_available: row.portsAvailable,
        has_upstream_relation: row.hasUpstreamRelation ? 1 : 0,
        has_customer_relation: row.hasCustomerRelation ? 1 : 0,
      },
    })),
  };
}

export function buildTopologyAssetAutoLinkFeatureCollection(args: {
  assets: NetworkAssetListItem[];
  topologyRows: TopologyAssetRow[];
  customerNodes: NMNode[];
  nodeRows: NMNode[];
  linkRows: NMLink[];
}): FeatureCollection {
  const rowsById = new Map(args.topologyRows.map((row) => [row.id, row]));
  const customerNodes = (args.customerNodes || []).filter(
    (node) => isCustomerNodeType(node.node_type) && isActiveCustomerNode(node),
  );
  const assetNodeIds = new Map<string, string>();
  const existingPairs = new Set<string>();
  const features: FeatureCollection['features'] = [];
  const linkIds = new Set<string>();

  for (const node of args.nodeRows || []) {
    const assetSource = String(node.metadata?.asset_source || node.metadata?.asset_type || '').trim();
    const assetId = String(node.metadata?.asset_id || '').trim();
    if (assetSource === 'network_asset' && assetId) {
      assetNodeIds.set(assetId, node.id);
    }
  }

  for (const row of args.linkRows || []) {
    const fromNodeId = String(row.from_node_id || '').trim();
    const toNodeId = String(row.to_node_id || '').trim();
    if (!fromNodeId || !toNodeId) continue;
    existingPairs.add(buildNodePairKey(fromNodeId, toNodeId));
  }

  for (const row of args.topologyRows) {
    if (row.parentAssetId) {
      const parent = rowsById.get(row.parentAssetId);
      const parentNodeId = assetNodeIds.get(parent?.id || '');
      const childNodeId = assetNodeIds.get(row.id);
      if (
        parent &&
        !hasExistingRealLink(existingPairs, parentNodeId, childNodeId)
      ) {
        pushLineFeature(features, linkIds, {
          id: `asset-parent:${parent.id}:${row.id}`,
          name: `${parent.name} -> ${row.name}`,
          kind: 'asset_parent',
          from: [parent.longitude, parent.latitude],
          to: [row.longitude, row.latitude],
        });
      }
    }
  }

  for (const row of args.topologyRows) {
    if (row.assetType !== 'odp') continue;
    const endpointKeys = new Set<string>();
    const sourceNodeId = assetNodeIds.get(row.id);
    const directCustomerNode = resolveCustomerNode(customerNodes, row.locationId, row.customerId);
    if (directCustomerNode) {
      const endpointKey = `${row.id}:${String(row.locationId || row.customerId || row.id)}`;
      endpointKeys.add(endpointKey);
      if (
        !hasExistingRealLink(existingPairs, sourceNodeId, directCustomerNode.id) &&
        !hasExistingCustomerSemanticLink(args.linkRows, args.nodeRows, sourceNodeId, row.locationId, row.customerId) &&
        !hasExistingCustomerDropGeometry(
          args.linkRows,
          [row.longitude, row.latitude],
          [directCustomerNode.lng, directCustomerNode.lat],
        )
      ) {
        pushLineFeature(features, linkIds, {
          id: `customer-drop:${row.id}:${directCustomerNode.id}`,
          name: `${row.name} -> ${directCustomerNode.name}`,
          kind: 'customer_drop',
          from: [row.longitude, row.latitude],
          to: [directCustomerNode.lng, directCustomerNode.lat],
        });
      }
    }

    for (const asset of args.assets) {
      if (asset.parent_asset_id !== row.id) continue;
      if (asset.status === 'faulty' || asset.status === 'retired') continue;
      const customerNode = resolveCustomerNode(customerNodes, asset.location_id, asset.customer_id);
      if (!customerNode) continue;
      const endpointKey = `${row.id}:${String(asset.location_id || asset.customer_id || asset.id)}`;
      if (endpointKeys.has(endpointKey)) continue;
      endpointKeys.add(endpointKey);
      if (
        !hasExistingRealLink(existingPairs, sourceNodeId, customerNode.id) &&
        !hasExistingCustomerSemanticLink(
          args.linkRows,
          args.nodeRows,
          sourceNodeId,
          asset.location_id,
          asset.customer_id,
        ) &&
        !hasExistingCustomerDropGeometry(
          args.linkRows,
          [row.longitude, row.latitude],
          [customerNode.lng, customerNode.lat],
        )
      ) {
        pushLineFeature(features, linkIds, {
          id: `customer-drop:${row.id}:${customerNode.id}`,
          name: `${row.name} -> ${customerNode.name}`,
          kind: 'customer_drop',
          from: [row.longitude, row.latitude],
          to: [customerNode.lng, customerNode.lat],
        });
      }
    }
  }

  return {
    type: 'FeatureCollection',
    features,
  };
}

function resolveCustomerNode(
  nodes: NMNode[],
  locationId: string | null,
  customerId: string | null,
) {
  const normalizedLocationId = String(locationId || '').trim();
  if (normalizedLocationId) {
    const matched = nodes.find(
      (node) => String(node.metadata?.location_id || '').trim() === normalizedLocationId,
    );
    if (matched) return matched;
  }

  const normalizedCustomerId = String(customerId || '').trim();
  if (normalizedCustomerId) {
    return (
      nodes.find((node) => String(node.metadata?.customer_id || '').trim() === normalizedCustomerId) ||
      null
    );
  }

  return null;
}

function buildNodePairKey(a: string, b: string) {
  return [a, b].sort().join('::');
}

function hasExistingRealLink(existingPairs: Set<string>, a?: string | null, b?: string | null) {
  const from = String(a || '').trim();
  const to = String(b || '').trim();
  if (!from || !to) return false;
  return existingPairs.has(buildNodePairKey(from, to));
}

function hasExistingCustomerSemanticLink(
  linkRows: NMLink[],
  nodeRows: NMNode[],
  sourceNodeId: string | undefined,
  locationId: string | null,
  customerId: string | null,
) {
  const normalizedSourceNodeId = String(sourceNodeId || '').trim();
  const normalizedLocationId = String(locationId || '').trim();
  const normalizedCustomerId = String(customerId || '').trim();
  if (!normalizedSourceNodeId || (!normalizedLocationId && !normalizedCustomerId)) return false;

  return (linkRows || []).some((row) => {
    const fromNodeId = String(row.from_node_id || '').trim();
    const toNodeId = String(row.to_node_id || '').trim();
    let otherNodeId = '';
    if (fromNodeId === normalizedSourceNodeId) {
      otherNodeId = toNodeId;
    } else if (toNodeId === normalizedSourceNodeId) {
      otherNodeId = fromNodeId;
    }
    if (!otherNodeId) return false;

    const otherNode = (nodeRows || []).find((node) => node.id === otherNodeId);
    if (!otherNode || !isCustomerNodeType(otherNode.node_type)) return false;

    const otherLocationId = String(otherNode.metadata?.location_id || '').trim();
    if (normalizedLocationId && otherLocationId === normalizedLocationId) return true;

    const otherCustomerId = String(otherNode.metadata?.customer_id || '').trim();
    return !!normalizedCustomerId && otherCustomerId === normalizedCustomerId;
  });
}

function hasExistingCustomerDropGeometry(
  linkRows: NMLink[],
  sourceCoord: [number, number],
  targetCoord: [number, number],
) {
  return (linkRows || []).some((row) => {
    const endpoints = extractLinkEndpoints(row.geometry);
    if (!endpoints) return false;
    return (
      (coordsAlmostEqual(endpoints.from, sourceCoord) && coordsAlmostEqual(endpoints.to, targetCoord)) ||
      (coordsAlmostEqual(endpoints.from, targetCoord) && coordsAlmostEqual(endpoints.to, sourceCoord))
    );
  });
}

function extractLinkEndpoints(geometry: GeoJSON.Geometry | null | undefined) {
  if (!geometry) return null;

  if (geometry.type === 'LineString') {
    const coordinates = geometry.coordinates;
    if (!Array.isArray(coordinates) || coordinates.length < 2) return null;
    const from = toLngLatTuple(coordinates[0]);
    const to = toLngLatTuple(coordinates[coordinates.length - 1]);
    if (!from || !to) return null;
    return { from, to };
  }

  if (geometry.type === 'MultiLineString') {
    const segments = geometry.coordinates;
    if (!Array.isArray(segments) || segments.length === 0) return null;
    const firstSegment = segments.find((segment) => Array.isArray(segment) && segment.length > 0);
    const lastSegment = [...segments].reverse().find((segment) => Array.isArray(segment) && segment.length > 0);
    if (!firstSegment || !lastSegment) return null;
    const from = toLngLatTuple(firstSegment[0]);
    const to = toLngLatTuple(lastSegment[lastSegment.length - 1]);
    if (!from || !to) return null;
    return { from, to };
  }

  return null;
}

function toLngLatTuple(value: unknown): [number, number] | null {
  if (!Array.isArray(value) || value.length < 2) return null;
  const lng = Number(value[0]);
  const lat = Number(value[1]);
  if (!Number.isFinite(lng) || !Number.isFinite(lat)) return null;
  return [lng, lat];
}

function coordsAlmostEqual(a: [number, number], b: [number, number], epsilon = 0.00001) {
  return Math.abs(a[0] - b[0]) <= epsilon && Math.abs(a[1] - b[1]) <= epsilon;
}

function pushLineFeature(
  features: FeatureCollection['features'],
  linkIds: Set<string>,
  args: {
    id: string;
    name: string;
    kind: 'asset_parent' | 'customer_drop';
    from: [number, number];
    to: [number, number];
  },
) {
  if (linkIds.has(args.id)) return;
  linkIds.add(args.id);
  features.push({
    type: 'Feature',
    geometry: {
      type: 'LineString',
      coordinates: [args.from, args.to],
    },
    properties: {
      id: args.id,
      name: args.name,
      link_kind: args.kind,
      status: 'up',
    },
  });
}

function hasTopologyAssetCustomerRelation(
  asset: Pick<NetworkAssetListItem, 'id' | 'asset_type' | 'customer_id' | 'location_id'>,
  allAssets: Pick<
    NetworkAssetListItem,
    'id' | 'asset_type' | 'parent_asset_id' | 'status' | 'customer_id' | 'location_id'
  >[],
  topology?: NetworkAssetOccupancyTopologyArgs,
): boolean {
  if (String(asset.location_id || '').trim() || String(asset.customer_id || '').trim()) {
    return true;
  }

  for (const item of allAssets) {
    if (item.parent_asset_id !== asset.id) continue;
    if (item.status === 'faulty' || item.status === 'retired') continue;
    if (String(item.location_id || '').trim() || String(item.customer_id || '').trim()) {
      return true;
    }
  }

  const sourceNodeId =
    topology?.assetNodeIdsByAssetId?.get(asset.id) ||
    (topology?.nodeRows || []).find((node) => {
      const source = String(node.metadata?.asset_source || node.metadata?.asset_type || '').trim();
      const assetId = String(node.metadata?.asset_id || '').trim();
      return source === 'network_asset' && assetId === asset.id;
    })?.id;
  if (!sourceNodeId) return false;

  return (topology?.linkRows || []).some((link) => {
    const fromNodeId = String(link.from_node_id || '').trim();
    const toNodeId = String(link.to_node_id || '').trim();
    let otherNodeId = '';
    if (fromNodeId === sourceNodeId) otherNodeId = toNodeId;
    else if (toNodeId === sourceNodeId) otherNodeId = fromNodeId;
    if (!otherNodeId) return false;

    const otherNode = (topology?.nodeRows || []).find((node) => node.id === otherNodeId);
    return isLinkedCustomerNode(otherNode);
  });
}

function hasTopologyAssetUpstreamRelation(
  asset: Pick<NetworkAssetListItem, 'id' | 'asset_type' | 'parent_asset_id'>,
  allAssets: Pick<NetworkAssetListItem, 'id' | 'asset_type'>[],
  topology?: NetworkAssetOccupancyTopologyArgs,
): boolean {
  if (String(asset.parent_asset_id || '').trim()) {
    return true;
  }

  const assetType = String(asset.asset_type || '').trim();
  const assetRank = UPSTREAM_RANK[assetType];
  if (!Number.isFinite(assetRank)) return false;

  const sourceNodeId =
    topology?.assetNodeIdsByAssetId?.get(asset.id) ||
    (topology?.nodeRows || []).find((node) => {
      const source = String(node.metadata?.asset_source || node.metadata?.asset_type || '').trim();
      const assetId = String(node.metadata?.asset_id || '').trim();
      return source === 'network_asset' && assetId === asset.id;
    })?.id;
  if (!sourceNodeId) return false;

  return (topology?.linkRows || []).some((link) => {
    const fromNodeId = String(link.from_node_id || '').trim();
    const toNodeId = String(link.to_node_id || '').trim();
    let otherNodeId = '';
    if (fromNodeId === sourceNodeId) otherNodeId = toNodeId;
    else if (toNodeId === sourceNodeId) otherNodeId = fromNodeId;
    if (!otherNodeId) return false;

    const otherNode = (topology?.nodeRows || []).find((node) => node.id === otherNodeId);
    const otherSource = String(
      otherNode?.metadata?.asset_source || otherNode?.metadata?.asset_type || '',
    ).trim();
    if (otherSource !== 'network_asset') return false;

    const otherAssetId = String(otherNode?.metadata?.asset_id || '').trim();
    const otherAssetType =
      String(otherNode?.metadata?.asset_type || '').trim() ||
      String(allAssets.find((item) => item.id === otherAssetId)?.asset_type || '').trim();
    const otherRank = UPSTREAM_RANK[otherAssetType];
    return Number.isFinite(otherRank) && otherRank < assetRank;
  });
}

function isCustomerLocationNode(node: Pick<NMNode, 'metadata'> | undefined) {
  const source = String(node?.metadata?.asset_source || node?.metadata?.asset_type || '').trim();
  return source === 'customer_location';
}

function isLinkedCustomerLocationNode(
  node: Pick<NMNode, 'node_type' | 'metadata'> | undefined,
) {
  return isCustomerLocationNode(node) && String(node?.node_type || '').trim() === 'customer_premise';
}

function isLinkedCustomerNode(
  node: Pick<NMNode, 'node_type' | 'metadata'> | undefined,
) {
  return isCustomerLocationNode(node) && isCustomerNodeType(String(node?.node_type || '').trim());
}

function isActiveCustomerNode(node: Pick<NMNode, 'status'> | undefined) {
  const status = String(node?.status || '').trim().toLowerCase();
  return status === 'active' || status === 'up';
}
