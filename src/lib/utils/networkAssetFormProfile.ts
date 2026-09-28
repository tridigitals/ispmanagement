type NetworkAssetFormField =
  | 'type'
  | 'status'
  | 'name'
  | 'code'
  | 'serial_number'
  | 'vendor'
  | 'model';

type NetworkAssetFormProfile = {
  group: 'distribution' | 'endpoint_device' | 'active_device' | 'power' | 'generic';
  identityFields: NetworkAssetFormField[];
  hardwareFieldsInline: NetworkAssetFormField[];
  hardwareFieldsOptional: NetworkAssetFormField[];
  detailSectionTitle: string;
  detailSectionKicker: string;
};

const DISTRIBUTION_ASSET_TYPES = new Set(['olt', 'odc', 'odp', 'splitter', 'fat', 'nap', 'odf']);
const ENDPOINT_DEVICE_TYPES = new Set(['ont', 'onu', 'media_converter']);
const ACTIVE_DEVICE_TYPES = new Set(['switch', 'router']);
const POWER_TYPES = new Set(['ups']);

export function getNetworkAssetFormProfile(assetType: string): NetworkAssetFormProfile {
  const normalizedType = String(assetType || '').trim();

  if (DISTRIBUTION_ASSET_TYPES.has(normalizedType)) {
    return {
      group: 'distribution',
      identityFields: ['type', 'status', 'name', 'code'],
      hardwareFieldsInline: [],
      hardwareFieldsOptional: ['serial_number', 'vendor', 'model'],
      detailSectionTitle: 'Topology Profile & Capacity',
      detailSectionKicker: 'Topology, Port & Coverage',
    };
  }

  if (ENDPOINT_DEVICE_TYPES.has(normalizedType)) {
    return {
      group: 'endpoint_device',
      identityFields: ['type', 'status', 'name', 'code'],
      hardwareFieldsInline: ['serial_number', 'vendor', 'model'],
      hardwareFieldsOptional: [],
      detailSectionTitle: 'Subscriber Device Details',
      detailSectionKicker: 'Provisioning & Device Identity',
    };
  }

  if (ACTIVE_DEVICE_TYPES.has(normalizedType)) {
    return {
      group: 'active_device',
      identityFields: ['type', 'status', 'name', 'code'],
      hardwareFieldsInline: ['serial_number', 'vendor', 'model'],
      hardwareFieldsOptional: [],
      detailSectionTitle: 'Device Profile & Capacity',
      detailSectionKicker: 'Management & Network Role',
    };
  }

  if (POWER_TYPES.has(normalizedType)) {
    return {
      group: 'power',
      identityFields: ['type', 'status', 'name', 'code'],
      hardwareFieldsInline: ['serial_number', 'vendor', 'model'],
      hardwareFieldsOptional: [],
      detailSectionTitle: 'Power Profile & Capacity',
      detailSectionKicker: 'Electrical & Backup Details',
    };
  }

  return {
    group: 'generic',
    identityFields: ['type', 'status', 'name', 'code'],
    hardwareFieldsInline: ['serial_number', 'vendor', 'model'],
    hardwareFieldsOptional: [],
    detailSectionTitle: 'Asset-specific details',
    detailSectionKicker: 'Asset Detail & Capacity',
  };
}
