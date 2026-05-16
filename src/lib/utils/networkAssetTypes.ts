import type { NetworkAssetGroup, NetworkAssetStatus, NetworkAssetType } from '$lib/api/client';

export type NetworkAssetTypeGroup = {
  label: string;
  types: NetworkAssetType[];
};

export const NETWORK_ASSET_TYPES: NetworkAssetType[] = [
  'olt',
  'odc',
  'fat',
  'odp',
  'nap',
  'splitter',
  'odf',
  'ont',
  'onu',
  'media_converter',
  'switch',
  'router',
  'ups',
];

export const NETWORK_ASSET_TYPE_GROUPS: NetworkAssetTypeGroup[] = [
  {
    label: 'FTTH Distribution',
    types: ['olt', 'odc', 'fat', 'odp', 'nap', 'splitter', 'odf'],
  },
  {
    label: 'Customer Endpoint',
    types: ['ont', 'onu', 'media_converter'],
  },
  {
    label: 'Infrastructure Device',
    types: ['switch', 'router', 'ups'],
  },
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
