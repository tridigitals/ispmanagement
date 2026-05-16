import type { NetworkAssetListItem } from '$lib/api/client';
import type { NMLink, NMNode } from './networkMapUtils';

export function buildTopologyAssetConnectionItems(args: {
  asset: Pick<
    NetworkAssetListItem,
    'id' | 'asset_type' | 'parent_asset_name' | 'customer_name' | 'location_label'
  >;
  topologyAssets: Pick<NetworkAssetListItem, 'id' | 'name'>[];
  assetNodeIdsByAssetId: Map<string, string>;
  nodeRows: NMNode[];
  linkRows: NMLink[];
}) {
  const sourceNodeId =
    args.assetNodeIdsByAssetId.get(args.asset.id) ||
    args.nodeRows.find((node) => {
      const source = String(node.metadata?.asset_source || node.metadata?.asset_type || '').trim();
      const assetId = String(node.metadata?.asset_id || '').trim();
      return source === 'network_asset' && assetId === args.asset.id;
    })?.id ||
    '';
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
