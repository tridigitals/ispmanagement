import type { NetworkAssetListItem } from '$lib/api/client';
import { isCustomerNodeType, type NMLink, type NMNode } from './networkMapUtils';

export type TopologyAssetCustomerDropItem = {
  key: string;
  customerId: string;
  serviceId: string;
  customerName: string;
  serviceName: string;
  locationLabel: string;
  status: string;
  nodeId: string;
};

function resolveCustomerNode(nodes: NMNode[], locationId?: string | null, customerId?: string | null) {
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

function pushCustomerDropItem(
  items: Map<string, TopologyAssetCustomerDropItem>,
  otherNode: NMNode,
  fallbackKey?: string,
) {
  const customerId = String(otherNode.metadata?.customer_id || '').trim();
  const serviceId = String(
    otherNode.metadata?.service_id || otherNode.metadata?.subscription_id || '',
  ).trim();
  const customerName = String(
    otherNode.metadata?.customer_name || otherNode.metadata?.customer_label || otherNode.name || '',
  ).trim();
  const serviceName = String(
    otherNode.metadata?.service_name ||
      otherNode.metadata?.service_label ||
      otherNode.metadata?.package_name ||
      '',
  ).trim();
  const locationLabel = String(
    otherNode.metadata?.location_label || otherNode.metadata?.address_label || '',
  ).trim();
  const status = String(
    otherNode.metadata?.subscription_status || otherNode.metadata?.service_status || otherNode.status || '',
  ).trim();
  const key =
    String(otherNode.metadata?.location_id || '').trim() ||
    customerId ||
    serviceId ||
    fallbackKey ||
    otherNode.id;

  if (!key || items.has(key)) return;

  items.set(key, {
    key,
    customerId,
    serviceId,
    customerName: customerName || locationLabel || otherNode.name || '-',
    serviceName,
    locationLabel,
    status: status || '-',
    nodeId: otherNode.id,
  });
}

function resolveAssetSourceNodeId(args: {
  assetId: string;
  assetNodeIdsByAssetId: Map<string, string>;
  nodeRows: NMNode[];
}) {
  return (
    args.assetNodeIdsByAssetId.get(args.assetId) ||
    args.nodeRows.find((node) => {
      const source = String(node.metadata?.asset_source || node.metadata?.asset_type || '').trim();
      const assetId = String(node.metadata?.asset_id || '').trim();
      return source === 'network_asset' && assetId === args.assetId;
    })?.id ||
    ''
  );
}

export function buildTopologyAssetCustomerDropItems(args: {
  assetId: string;
  assets?: Array<
    Pick<
      NetworkAssetListItem,
      'id' | 'parent_asset_id' | 'customer_id' | 'location_id' | 'customer_name' | 'location_label' | 'status'
    >
  >;
  assetNodeIdsByAssetId: Map<string, string>;
  nodeRows: NMNode[];
  linkRows: NMLink[];
}) {
  const sourceNodeId = resolveAssetSourceNodeId({
    assetId: args.assetId,
    assetNodeIdsByAssetId: args.assetNodeIdsByAssetId,
    nodeRows: args.nodeRows,
  });
  if (!sourceNodeId) return [] as TopologyAssetCustomerDropItem[];

  const items = new Map<string, TopologyAssetCustomerDropItem>();
  const customerNodes = (args.nodeRows || []).filter(
    (node) =>
      String(node.metadata?.asset_source || node.metadata?.asset_type || '').trim() === 'customer_location' ||
      isCustomerNodeType(String(node.node_type || '').trim()),
  );

  const directAsset = (args.assets || []).find((asset) => asset.id === args.assetId);
  if (directAsset) {
    const directCustomerNode = resolveCustomerNode(
      customerNodes,
      directAsset.location_id,
      directAsset.customer_id,
    );
    if (directCustomerNode) {
      pushCustomerDropItem(
        items,
        directCustomerNode,
        String(directAsset.location_id || directAsset.customer_id || directAsset.id),
      );
    }
  }

  for (const childAsset of args.assets || []) {
    if (childAsset.parent_asset_id !== args.assetId) continue;
    if (childAsset.status === 'faulty' || childAsset.status === 'retired') continue;
    const childCustomerNode = resolveCustomerNode(
      customerNodes,
      childAsset.location_id,
      childAsset.customer_id,
    );
    if (!childCustomerNode) continue;
    pushCustomerDropItem(
      items,
      childCustomerNode,
      String(childAsset.location_id || childAsset.customer_id || childAsset.id),
    );
  }

  for (const row of args.linkRows || []) {
    const fromNodeId = String(row.from_node_id || '').trim();
    const toNodeId = String(row.to_node_id || '').trim();
    let otherNodeId = '';
    if (fromNodeId === sourceNodeId) otherNodeId = toNodeId;
    else if (toNodeId === sourceNodeId) otherNodeId = fromNodeId;
    if (!otherNodeId) continue;

    const otherNode = (args.nodeRows || []).find((node) => node.id === otherNodeId);
    if (!otherNode) continue;

    const source = String(
      otherNode.metadata?.asset_source || otherNode.metadata?.asset_type || '',
    ).trim();
    if (source !== 'customer_location' && !isCustomerNodeType(String(otherNode.node_type || '').trim())) {
      continue;
    }

    pushCustomerDropItem(items, otherNode);
  }

  return Array.from(items.values());
}

export function buildTopologyAssetConnectionItems(args: {
  asset: Pick<
    NetworkAssetListItem,
    'id' | 'asset_type' | 'parent_asset_name' | 'customer_name' | 'location_label'
  >;
  topologyAssets: Pick<NetworkAssetListItem, 'id' | 'name'>[];
  assetNodeIdsByAssetId: Map<string, string>;
  nodeRows: NMNode[];
  linkRows: NMLink[];
  routerRows?: Array<{ id: string; name?: string | null; identity?: string | null }>;
}) {
  const sourceNodeId = resolveAssetSourceNodeId({
    assetId: args.asset.id,
    assetNodeIdsByAssetId: args.assetNodeIdsByAssetId,
    nodeRows: args.nodeRows,
  });
  if (!sourceNodeId) return [];

  const upstreamNames = new Set<string>();
  const customerNames: string[] = [];

  for (const row of args.linkRows || []) {
    const fromNodeId = String(row.from_node_id || '').trim();
    const toNodeId = String(row.to_node_id || '').trim();
    let otherNodeId = '';
    if (fromNodeId === sourceNodeId) otherNodeId = toNodeId;
    else if (toNodeId === sourceNodeId) otherNodeId = fromNodeId;
    if (!otherNodeId) continue;

    const linkedRouter = (args.routerRows || []).find((router) => String(router.id || '').trim() === otherNodeId);
    if (linkedRouter) {
      const name = String(linkedRouter.identity || linkedRouter.name || '').trim();
      if (name) upstreamNames.add(name);
      continue;
    }

    const otherNode = (args.nodeRows || []).find((node) => node.id === otherNodeId);
    if (!otherNode) continue;

    const source = String(
      otherNode.metadata?.asset_source || otherNode.metadata?.asset_type || '',
    ).trim();
    if (source === 'network_asset') {
      const assetId = String(otherNode.metadata?.asset_id || '').trim();
      const name =
        args.topologyAssets.find((asset) => asset.id === assetId)?.name ||
        String(otherNode.name || '').trim();
      if (name) upstreamNames.add(name);
      continue;
    }

    if (source !== 'customer_location' && !isCustomerNodeType(String(otherNode.node_type || '').trim())) {
      const name = String(otherNode.name || '').trim();
      if (name) upstreamNames.add(name);
      continue;
    }

    if (source === 'customer_location' || String(otherNode.node_type || '').trim() === 'customer_premise') {
      const customerName = String(
        otherNode.metadata?.customer_name ||
          otherNode.metadata?.customer_label ||
          otherNode.metadata?.location_label ||
          otherNode.name ||
          '',
      ).trim();
      if (customerName) customerNames.push(customerName);
    }
  }

  const items: Array<{ label: string; value: string }> = [];
  const upstream =
    Array.from(upstreamNames)[0] || String(args.asset.parent_asset_name || '').trim();
  if (upstream) {
    items.push({ label: 'Upstream', value: upstream });
  }

  if (customerNames.length > 0) {
    items.push({ label: 'Ports Used', value: `${customerNames.length} endpoint linked` });
    const visibleNames = customerNames.slice(0, 5);
    const hiddenCount = Math.max(customerNames.length - visibleNames.length, 0);
    const summary =
      hiddenCount > 0 ? `${visibleNames.join(', ')} +${hiddenCount} more` : visibleNames.join(', ');
    items.push({ label: 'Connected', value: summary });
  }

  return items;
}
