import type { CreateNetworkAssetRequest } from '$lib/api/types';
import type { NetworkAssetListItem } from '$lib/api/types';

export type InstallationQuickAssetDraft = {
  asset_type: 'ont' | 'onu';
  name: string;
  code: string;
  vendor: string;
  model: string;
  serial_number: string;
};

export function buildDefaultInstallationQuickAssetDraft(): InstallationQuickAssetDraft {
  return {
    asset_type: 'ont',
    name: '',
    code: '',
    vendor: '',
    model: '',
    serial_number: '',
  };
}

export function buildInstallationQuickAssetSuggestedName(input: {
  assetType: 'ont' | 'onu';
  customerName?: string | null;
  locationLabel?: string | null;
}): string {
  const prefix = input.assetType.toUpperCase();
  const customer = normalizeFreeText(input.customerName || '');
  const location = normalizeFreeText(input.locationLabel || '');
  if (customer && location) return `${prefix} ${customer} - ${location}`;
  if (customer) return `${prefix} ${customer}`;
  if (location) return `${prefix} ${location}`;
  return prefix;
}

export function syncInstallationQuickAssetDraftOnTypeChange(input: {
  draft: InstallationQuickAssetDraft;
  nextAssetType: 'ont' | 'onu';
  customerName?: string | null;
  locationLabel?: string | null;
}): InstallationQuickAssetDraft {
  const { draft, nextAssetType, customerName, locationLabel } = input;
  const currentSuggestedName = buildInstallationQuickAssetSuggestedName({
    assetType: draft.asset_type,
    customerName,
    locationLabel,
  });
  const nextSuggestedName = buildInstallationQuickAssetSuggestedName({
    assetType: nextAssetType,
    customerName,
    locationLabel,
  });

  return {
    ...draft,
    asset_type: nextAssetType,
    name:
      !draft.name.trim() || draft.name === currentSuggestedName ? nextSuggestedName : draft.name,
  };
}

export function applyInstallationQuickAssetInputChange(
  field: keyof InstallationQuickAssetDraft,
  value: string,
): string {
  if (field === 'code' || field === 'serial_number') {
    return normalizeIdentifier(value);
  }
  if (field === 'vendor') {
    return normalizeWords(value, true);
  }
  if (field === 'model') {
    return normalizeWords(value, true);
  }
  if (field === 'name') {
    return normalizeFreeText(value);
  }
  return value;
}

export function validateInstallationQuickAssetDraft(
  draft: InstallationQuickAssetDraft,
): string | null {
  if (!['ont', 'onu'].includes(draft.asset_type)) {
    return 'Quick create only supports ONT or ONU.';
  }
  if (!draft.name.trim()) {
    return 'Asset name is required.';
  }
  if (!draft.serial_number.trim()) {
    return 'Serial number is required.';
  }
  return null;
}

export function findInstallationQuickAssetDuplicates(
  draft: InstallationQuickAssetDraft,
  assets: Pick<NetworkAssetListItem, 'code' | 'serial_number'>[],
): {
  code?: string;
  serial_number?: string;
} {
  const normalizedCode = normalizeIdentifier(draft.code);
  const normalizedSerial = normalizeIdentifier(draft.serial_number);
  const duplicates: {
    code?: string;
    serial_number?: string;
  } = {};

  if (
    normalizedCode &&
    assets.some((asset) => normalizeIdentifier(asset.code || '') === normalizedCode)
  ) {
    duplicates.code = 'Asset code already exists in registry.';
  }

  if (
    normalizedSerial &&
    assets.some((asset) => normalizeIdentifier(asset.serial_number || '') === normalizedSerial)
  ) {
    duplicates.serial_number = 'Serial number already exists in registry.';
  }

  return duplicates;
}

export function buildInstallationQuickAssetPayload(input: {
  draft: InstallationQuickAssetDraft;
  customer_id: string;
  location_id: string;
  work_order_id: string;
  notes?: string | null;
}): CreateNetworkAssetRequest {
  const { draft } = input;
  return {
    asset_type: draft.asset_type,
    name: normalizeFreeText(draft.name),
    code: normalizeIdentifier(draft.code) || null,
    vendor: normalizeWords(draft.vendor, true) || null,
    model: normalizeWords(draft.model, true) || null,
    serial_number: normalizeIdentifier(draft.serial_number),
    status: 'available',
    customer_id: input.customer_id || null,
    location_id: input.location_id || null,
    work_order_id: input.work_order_id || null,
    parent_asset_id: null,
    notes: input.notes?.trim() || null,
    metadata: {},
  };
}

function normalizeIdentifier(value: string): string {
  return value
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9]+/g, '-')
    .replace(/-{2,}/g, '-')
    .replace(/^-|-$/g, '');
}

function normalizeWords(value: string, upper = false): string {
  const normalized = value.trim().replace(/\s+/g, ' ');
  return upper ? normalized.toUpperCase() : normalized;
}

function normalizeFreeText(value: string): string {
  return normalizeWords(value, false);
}
