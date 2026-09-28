import type { NetworkAssetListItem } from '$lib/api/client';

const DISTRIBUTION_ASSET_TYPES = new Set(['olt', 'odc', 'odp', 'fat', 'nap', 'splitter', 'odf']);

export type NetworkAssetConnectionItem = {
  label: string;
  value: string;
};

export function buildNetworkAssetConnectionItems(
  asset: Pick<
    NetworkAssetListItem,
    | 'id'
    | 'asset_type'
    | 'parent_asset_name'
    | 'customer_name'
    | 'location_label'
    | 'parent_asset_id'
  >,
  allAssets: Pick<
    NetworkAssetListItem,
    | 'id'
    | 'asset_type'
    | 'name'
    | 'status'
    | 'parent_asset_id'
    | 'parent_asset_name'
    | 'customer_name'
    | 'location_label'
  >[],
): NetworkAssetConnectionItem[] {
  const items: NetworkAssetConnectionItem[] = [];
  const upstream = String(asset.parent_asset_name || '').trim();
  if (upstream) {
    items.push({ label: 'Upstream', value: upstream });
  }

  if (DISTRIBUTION_ASSET_TYPES.has(String(asset.asset_type || '').trim())) {
    const connectedAssets = allAssets
      .filter((item) => item.parent_asset_id === asset.id)
      .filter((item) => !['retired', 'faulty'].includes(String(item.status || '').trim()));
    if (connectedAssets.length > 0) {
      items.push({
        label: 'Ports Used',
        value: `${connectedAssets.length} endpoint linked`,
      });

      const connectedNames = connectedAssets
        .map((item) => String(item.customer_name || item.location_label || item.name || '').trim())
        .filter(Boolean);
      const visibleNames = connectedNames.slice(0, 5);
      const hiddenCount = Math.max(connectedNames.length - visibleNames.length, 0);
      const summary =
        hiddenCount > 0
          ? `${visibleNames.join(', ')} +${hiddenCount} more`
          : visibleNames.join(', ');
      items.push({ label: 'Connected', value: summary });
    }
    return items;
  }

  const customer = String(asset.customer_name || '').trim();
  const location = String(asset.location_label || '').trim();
  const serviceTarget = [customer, location].filter(Boolean).join(' • ');
  if (serviceTarget) {
    items.push({ label: 'Service To', value: serviceTarget });
  }

  return items;
}
