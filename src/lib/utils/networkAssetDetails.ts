import type { NetworkAsset, NetworkAssetType } from '$lib/api/client';

export type NetworkAssetDetailField = {
  key: string;
  label: string;
  placeholder: string;
  inputMode?: 'text' | 'numeric' | 'decimal';
};

type DetailValidator = (value: string) => string | null;

const DETAIL_FIELDS: Record<string, NetworkAssetDetailField[]> = {
  olt: [
    { key: 'rack_slot', label: 'Rack / Slot', placeholder: 'Rack A / Slot 01' },
    { key: 'pon_capacity', label: 'PON Capacity', placeholder: '16 port', inputMode: 'numeric' },
    { key: 'uplink_capacity', label: 'Uplink Capacity', placeholder: '10G' },
  ],
  odc: [
    { key: 'cabinet_slot', label: 'Cabinet / Tray', placeholder: 'Tray 03' },
    { key: 'fiber_core_count', label: 'Fiber Core Count', placeholder: '144', inputMode: 'numeric' },
    { key: 'feeder_cable_code', label: 'Feeder Cable Code', placeholder: 'FDR-12A' },
  ],
  odp: [
    { key: 'total_port_capacity', label: 'Total Port Capacity', placeholder: '8', inputMode: 'numeric' },
    { key: 'splitter_ratio', label: 'Splitter Ratio', placeholder: '1:8' },
    { key: 'coverage_area', label: 'Coverage Area', placeholder: 'Blok C Timur' },
  ],
  splitter: [
    { key: 'splitter_ratio', label: 'Splitter Ratio', placeholder: '1:8' },
    { key: 'input_port', label: 'Input Port', placeholder: 'IN-01' },
    { key: 'output_ports', label: 'Output Ports', placeholder: '8', inputMode: 'numeric' },
  ],
  ont: [
    { key: 'pon_serial', label: 'PON Serial', placeholder: 'ZTEGC1234567' },
    { key: 'loid', label: 'LOID', placeholder: 'CUST-FTTH-001' },
    { key: 'mac_address', label: 'MAC Address', placeholder: 'AA:BB:CC:DD:EE:FF' },
  ],
  onu: [
    { key: 'pon_serial', label: 'PON Serial', placeholder: 'HWTC12345678' },
    { key: 'loid', label: 'LOID', placeholder: 'ONU-SITE-02' },
    { key: 'mac_address', label: 'MAC Address', placeholder: 'AA:BB:CC:DD:EE:FF' },
  ],
  fat: [
    { key: 'distribution_port', label: 'Distribution Port', placeholder: 'Port 12' },
    { key: 'fiber_core_count', label: 'Fiber Core Count', placeholder: '24', inputMode: 'numeric' },
    { key: 'coverage_area', label: 'Coverage Area', placeholder: 'Cluster Magnolia' },
  ],
  nap: [
    { key: 'distribution_port', label: 'Distribution Port', placeholder: 'Port 04' },
    { key: 'splitter_ratio', label: 'Splitter Ratio', placeholder: '1:4' },
    { key: 'coverage_area', label: 'Coverage Area', placeholder: 'Gang Anggrek' },
  ],
  switch: [
    { key: 'management_ip', label: 'Management IP', placeholder: '10.10.10.2' },
    { key: 'uplink_port', label: 'Uplink Port', placeholder: 'SFP1' },
    { key: 'firmware_version', label: 'Firmware Version', placeholder: 'v7.15.1' },
  ],
  router: [
    { key: 'management_ip', label: 'Management IP', placeholder: '10.10.1.1' },
    { key: 'uplink_port', label: 'WAN / Uplink Port', placeholder: 'ether1' },
    { key: 'firmware_version', label: 'Firmware Version', placeholder: 'RouterOS 7.15.3' },
  ],
  media_converter: [
    { key: 'management_ip', label: 'Management IP', placeholder: '10.10.20.5' },
    { key: 'optic_mode', label: 'Optic Mode', placeholder: 'Single mode' },
    { key: 'uplink_port', label: 'Uplink Port', placeholder: 'GE1 / SFP1' },
  ],
  odf: [
    { key: 'rack_slot', label: 'Rack / Panel', placeholder: 'Rack B / Panel 02' },
    { key: 'fiber_core_count', label: 'Fiber Core Count', placeholder: '48', inputMode: 'numeric' },
    { key: 'feeder_cable_code', label: 'Feeder Cable Code', placeholder: 'ODF-FDR-09' },
  ],
  ups: [
    { key: 'battery_capacity_ah', label: 'Battery Capacity (Ah)', placeholder: '100', inputMode: 'numeric' },
    { key: 'backup_runtime_minutes', label: 'Backup Runtime (Minutes)', placeholder: '90', inputMode: 'numeric' },
    { key: 'power_phase', label: 'Power Phase', placeholder: '1 phase' },
  ],
};

export type NetworkAssetDetailDraft = Record<string, string>;

export function getNetworkAssetDetailFields(assetType: string): NetworkAssetDetailField[] {
  return DETAIL_FIELDS[assetType] || [];
}

export function createNetworkAssetDetailDraft(
  assetType: string,
  metadata?: Record<string, unknown> | null,
): NetworkAssetDetailDraft {
  const fields = getNetworkAssetDetailFields(assetType);
  return Object.fromEntries(
    fields.map((field) => [field.key, getMetadataString(metadata, field.key)]),
  );
}

export function buildNetworkAssetMetadata(
  assetType: string,
  detailDraft: NetworkAssetDetailDraft,
  currentMetadata?: Record<string, unknown> | null,
): Record<string, unknown> {
  const next: Record<string, unknown> = { ...(currentMetadata || {}) };

  for (const fields of Object.values(DETAIL_FIELDS)) {
    for (const field of fields) {
      delete next[field.key];
    }
  }

  for (const field of getNetworkAssetDetailFields(assetType)) {
    const value = (detailDraft[field.key] || '').trim();
    if (value) {
      next[field.key] = value;
    } else {
      delete next[field.key];
    }
  }

  return next;
}

export function getNetworkAssetDetailSummary(asset: Pick<NetworkAsset, 'asset_type' | 'metadata'>): string[] {
  return getNetworkAssetDetailFields(asset.asset_type)
    .map((field) => {
      const value = getMetadataString(asset.metadata as Record<string, unknown> | null, field.key);
      if (!value) return null;
      return `${field.label}: ${value}`;
    })
    .filter((value): value is string => Boolean(value))
    .slice(0, 3);
}

export function validateNetworkAssetDetailDraft(
  assetType: string,
  detailDraft: NetworkAssetDetailDraft,
): string[] {
  return getNetworkAssetDetailFields(assetType)
    .map((field) => {
      const value = (detailDraft[field.key] || '').trim();
      if (!value) return null;
      const error = getFieldValidator(field.key)?.(value);
      if (!error) return null;
      return `${field.label}: ${error}`;
    })
    .filter((value): value is string => Boolean(value));
}

function getFieldValidator(key: string): DetailValidator | null {
  if (key === 'management_ip') return validateIpAddress;
  if (key === 'mac_address') return validateMacAddress;
  if (
    key === 'total_port_capacity' ||
    key === 'fiber_core_count' ||
    key === 'output_ports' ||
    key === 'battery_capacity_ah' ||
    key === 'backup_runtime_minutes'
  ) {
    return validatePositiveInteger;
  }
  return null;
}

function getMetadataString(
  metadata: Record<string, unknown> | null | undefined,
  key: string,
): string {
  const value = metadata?.[key];
  if (typeof value === 'string') {
    return value.trim();
  }
  if (typeof value === 'number') {
    return String(value);
  }
  return '';
}

function validateIpAddress(value: string): string | null {
  const ipv4Pattern =
    /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)$/;
  const ipv6Pattern = /^[0-9a-fA-F:]+$/;
  if (ipv4Pattern.test(value) || (value.includes(':') && ipv6Pattern.test(value))) {
    return null;
  }
  return 'must be a valid IP address';
}

function validateMacAddress(value: string): string | null {
  if (/^([0-9a-fA-F]{2}[:-]){5}[0-9a-fA-F]{2}$/.test(value)) {
    return null;
  }
  return 'must use MAC format like AA:BB:CC:DD:EE:FF';
}

function validatePositiveInteger(value: string): string | null {
  if (/^[1-9]\d*$/.test(value)) {
    return null;
  }
  return 'must be a positive whole number';
}

export const NETWORK_ASSET_DETAIL_TYPES = Object.keys(DETAIL_FIELDS) as NetworkAssetType[];
