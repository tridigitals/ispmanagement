import type { InstallationWorkOrderView, NetworkAssetListItem } from '$lib/api/types';
import { getNetworkAssetTypeLabel } from '$lib/utils/networkAssetTypes';
import { buildNetworkAssetOccupancyLabel } from '$lib/utils/networkAssetOccupancy';

export type InstallationAssetOption = {
  value: string;
  label: string;
};

export type InstallationAssetBindingState = {
  terminal_asset_id: string;
  parent_asset_id: string;
};

const TERMINAL_TYPES = new Set(['ont', 'onu']);
const PARENT_TYPES = new Set(['olt', 'odc', 'odp', 'splitter', 'fat', 'nap', 'odf']);
const BLOCKED_STATUSES = new Set(['faulty', 'retired']);

export function buildEmptyInstallationAssetBinding(): InstallationAssetBindingState {
  return {
    terminal_asset_id: '',
    parent_asset_id: '',
  };
}

export function buildInstallationTerminalAssetOptions(
  assets: NetworkAssetListItem[],
  row: Pick<InstallationWorkOrderView, 'id' | 'customer_id'>,
  selectedId?: string | null,
): InstallationAssetOption[] {
  return assets
    .filter((asset) => TERMINAL_TYPES.has(asset.asset_type))
    .filter((asset) => !BLOCKED_STATUSES.has(asset.status))
    .filter((asset) => {
      if (asset.id === selectedId) return true;
      if (asset.work_order_id === row.id) return true;
      if (asset.customer_id === row.customer_id) return true;
      return !asset.customer_id;
    })
    .sort(sortAssets)
    .map((asset) => buildAssetOption(asset));
}

export function buildInstallationParentAssetOptions(
  assets: NetworkAssetListItem[],
  selectedId?: string | null,
): InstallationAssetOption[] {
  return assets
    .filter((asset) => PARENT_TYPES.has(asset.asset_type))
    .filter((asset) => !BLOCKED_STATUSES.has(asset.status))
    .filter((asset) => asset.id === selectedId || asset.status !== 'retired')
    .sort(sortAssets)
    .map((asset) => buildAssetOption(asset, assets));
}

export function resolveInstallationAssetBinding(
  assets: NetworkAssetListItem[],
  workOrderId: string,
): InstallationAssetBindingState {
  const binding = buildEmptyInstallationAssetBinding();
  for (const asset of assets) {
    if (asset.work_order_id !== workOrderId) continue;
    if (!binding.terminal_asset_id && TERMINAL_TYPES.has(asset.asset_type)) {
      binding.terminal_asset_id = asset.id;
    }
    if (!binding.parent_asset_id && PARENT_TYPES.has(asset.asset_type)) {
      binding.parent_asset_id = asset.id;
    }
  }
  return binding;
}

export function validateInstallationAssetBinding(
  row: Pick<InstallationWorkOrderView, 'status'>,
  binding: InstallationAssetBindingState,
): string | null {
  if (row.status === 'in_progress' && !binding.terminal_asset_id.trim()) {
    return 'Select ONT/ONU asset before completion.';
  }
  return null;
}

function buildAssetOption(
  asset: NetworkAssetListItem,
  allAssets?: NetworkAssetListItem[],
): InstallationAssetOption {
  const parts = [asset.name, getNetworkAssetTypeLabel(asset.asset_type)];
  const occupancyLabel = allAssets ? buildNetworkAssetOccupancyLabel(asset, allAssets) : null;
  if (occupancyLabel) parts.push(occupancyLabel);
  if (asset.serial_number) parts.push(asset.serial_number);
  else if (asset.code) parts.push(asset.code);
  return {
    value: asset.id,
    label: parts.join(' • '),
  };
}

function sortAssets(a: NetworkAssetListItem, b: NetworkAssetListItem) {
  return a.name.localeCompare(b.name) || a.asset_type.localeCompare(b.asset_type);
}
