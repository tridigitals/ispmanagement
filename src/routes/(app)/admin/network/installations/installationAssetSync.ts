import type {
  InstallationWorkOrderView,
  NetworkAssetListItem,
  UpdateNetworkAssetRequest,
} from '$lib/api/types';

import type { InstallationAssetBindingState } from './installationAssetBinding';

const TERMINAL_TYPES = new Set(['ont', 'onu']);
const PARENT_TYPES = new Set(['olt', 'odc', 'odp', 'splitter', 'fat', 'nap', 'odf']);

export type InstallationAssetSyncUpdate = {
  id: string;
  payload: UpdateNetworkAssetRequest;
};

export function buildInstallationAssetSyncUpdates(args: {
  assets: NetworkAssetListItem[];
  row: Pick<InstallationWorkOrderView, 'id' | 'customer_id' | 'location_id'>;
  binding: InstallationAssetBindingState;
}): InstallationAssetSyncUpdate[] {
  const updates: InstallationAssetSyncUpdate[] = [];
  const selectedTerminalId = args.binding.terminal_asset_id.trim();
  const selectedParentId = args.binding.parent_asset_id.trim();

  const selectedTerminal = args.assets.find((asset) => asset.id === selectedTerminalId);
  if (selectedTerminal) {
    pushUpdateIfChanged(updates, selectedTerminal, {
      customer_id: args.row.customer_id,
      location_id: args.row.location_id,
      work_order_id: args.row.id,
      parent_asset_id: selectedParentId || null,
      status: 'reserved',
    });
  }

  for (const asset of args.assets) {
    if (!TERMINAL_TYPES.has(asset.asset_type)) continue;
    if (asset.id === selectedTerminalId) continue;
    if (asset.work_order_id !== args.row.id) continue;

    pushUpdateIfChanged(updates, asset, {
      customer_id: null,
      location_id: null,
      work_order_id: null,
      parent_asset_id: null,
      status: 'available',
    });
  }

  if (selectedParentId) {
    const selectedParent = args.assets.find((asset) => asset.id === selectedParentId);
    if (selectedParent) {
      pushUpdateIfChanged(updates, selectedParent, {
        work_order_id: args.row.id,
      });
    }
  }

  for (const asset of args.assets) {
    if (!PARENT_TYPES.has(asset.asset_type)) continue;
    if (asset.id === selectedParentId) continue;
    if (asset.work_order_id !== args.row.id) continue;

    pushUpdateIfChanged(updates, asset, {
      work_order_id: null,
    });
  }

  return updates;
}

function pushUpdateIfChanged(
  updates: InstallationAssetSyncUpdate[],
  asset: NetworkAssetListItem,
  payload: UpdateNetworkAssetRequest,
) {
  const nextPayload: UpdateNetworkAssetRequest = {};

  if (payload.customer_id !== undefined && payload.customer_id !== asset.customer_id) {
    nextPayload.customer_id = payload.customer_id;
  }
  if (payload.location_id !== undefined && payload.location_id !== asset.location_id) {
    nextPayload.location_id = payload.location_id;
  }
  if (payload.work_order_id !== undefined && payload.work_order_id !== asset.work_order_id) {
    nextPayload.work_order_id = payload.work_order_id;
  }
  if (payload.parent_asset_id !== undefined && payload.parent_asset_id !== asset.parent_asset_id) {
    nextPayload.parent_asset_id = payload.parent_asset_id;
  }
  if (payload.status !== undefined && payload.status !== asset.status) {
    nextPayload.status = payload.status;
  }

  if (Object.keys(nextPayload).length === 0) return;
  updates.push({
    id: asset.id,
    payload: nextPayload,
  });
}
