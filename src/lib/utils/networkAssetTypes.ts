import type { NetworkAssetGroup, NetworkAssetStatus, NetworkAssetType } from '$lib/api/client';

export const NETWORK_ASSET_TYPES: NetworkAssetType[] = [
  'olt',
  'odc',
  'odp',
  'splitter',
  'ont',
  'onu',
  'fat',
  'nap',
  'switch',
  'router',
  'media_converter',
  'odf',
  'ups',
];

export const NETWORK_ASSET_STATUSES: NetworkAssetStatus[] = [
  'available',
  'reserved',
  'installed',
  'faulty',
  'retired',
];

export function getNetworkAssetTypeLabel(type: string): string {
  if (type === 'olt') return 'OLT';
  if (type === 'odc') return 'ODC';
  if (type === 'odp') return 'ODP';
  if (type === 'ont') return 'ONT';
  if (type === 'onu') return 'ONU';
  if (type === 'fat') return 'FAT';
  if (type === 'nap') return 'NAP';
  if (type === 'switch') return 'Switch';
  if (type === 'router') return 'Router';
  if (type === 'media_converter') return 'Media Converter';
  if (type === 'odf') return 'ODF';
  if (type === 'ups') return 'UPS';
  if (type === 'splitter') return 'Splitter';
  return type;
}

export function getNetworkAssetGroup(type: string): NetworkAssetGroup {
  if (['switch', 'router', 'media_converter', 'odf', 'ups'].includes(type)) {
    return 'infrastructure';
  }
  return 'access_fiber';
}

export function getNetworkAssetGroupLabel(group: string): string {
  if (group === 'access_fiber') return 'Access Fiber';
  if (group === 'infrastructure') return 'Infrastructure';
  return group;
}

export function getNetworkAssetStatusLabel(status: string): string {
  if (status === 'available') return 'Available';
  if (status === 'reserved') return 'Reserved';
  if (status === 'installed') return 'Installed';
  if (status === 'faulty') return 'Faulty';
  if (status === 'retired') return 'Retired';
  return status;
}

export function getDefaultNetworkAssetStatus(): NetworkAssetStatus {
  return 'available';
}
