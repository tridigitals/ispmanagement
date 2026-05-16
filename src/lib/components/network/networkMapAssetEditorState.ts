import type { NetworkAssetListItem } from '$lib/api/client';
import {
  createNetworkAssetDetailDraft,
  type NetworkAssetDetailDraft,
} from '$lib/utils/networkAssetDetails';

export type NetworkMapAssetDraft = {
  asset_type: string;
  name: string;
  code: string;
  vendor: string;
  model: string;
  serial_number: string;
  status: string;
  latitude: string;
  longitude: string;
  notes: string;
};

export function buildNetworkMapAssetEditorState(asset: NetworkAssetListItem): {
  draft: NetworkMapAssetDraft;
  detailDraft: NetworkAssetDetailDraft;
} {
  return {
    draft: {
      asset_type: asset.asset_type,
      name: asset.name,
      code: asset.code || '',
      vendor: asset.vendor || '',
      model: asset.model || '',
      serial_number: asset.serial_number || '',
      status: asset.status,
      latitude: asset.latitude != null ? String(asset.latitude) : '',
      longitude: asset.longitude != null ? String(asset.longitude) : '',
      notes: asset.notes || '',
    },
    detailDraft: createNetworkAssetDetailDraft(asset.asset_type, asset.metadata || {}),
  };
}
