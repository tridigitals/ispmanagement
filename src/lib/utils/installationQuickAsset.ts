import type { CreateNetworkAssetRequest } from '$lib/api/types';
import type { NetworkAssetListItem } from '$lib/api/types';

/** Tipe aset yang boleh dibuat cepat dari halaman instalasi. */
export type InstallationQuickAssetType = 'ont' | 'onu' | 'odp';

export const INSTALLATION_QUICK_ASSET_TYPES: InstallationQuickAssetType[] = ['ont', 'onu', 'odp'];

export type InstallationQuickAssetDraft = {
  asset_type: InstallationQuickAssetType;
  name: string;
  code: string;
  vendor: string;
  model: string;
  serial_number: string;
  latitude: string;
  longitude: string;
  /** ODP induk (node FTTH) — kosongkan bila tidak ada. */
  parent_asset_id: string;
};

/**
 * Konteks instalasi yang dipakai untuk mengisi form otomatis.
 *
 * Semua nilai datang dari data work order yang sudah ada — tidak ada yang perlu
 * diketik ulang oleh teknisi/admin:
 *   - customer_name / location_label → untuk nama aset yang menyesuaikan lokasi
 *   - location_latitude/longitude    → lokasi aset (dulu selalu diisi manual)
 *   - location_code                  → usulan kode aset
 */
export type InstallationQuickAssetContext = {
  customerName?: string | null;
  locationLabel?: string | null;
  locationLatitude?: number | null;
  locationLongitude?: number | null;
  /** Kode lokasi bila ada; jadi usulan kode aset. */
  locationCode?: string | null;
};

export function buildDefaultInstallationQuickAssetDraft(): InstallationQuickAssetDraft {
  return {
    asset_type: 'ont',
    name: '',
    code: '',
    vendor: '',
    model: '',
    serial_number: '',
    latitude: '',
    longitude: '',
    parent_asset_id: '',
  };
}

export function buildInstallationQuickAssetSuggestedName(input: {
  assetType: InstallationQuickAssetType;
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

/**
 * Isi draf dari data instalasi yang sedang dibuka.
 *
 * Inilah "ambil dari data instalasi": nama aset mengikuti pelanggan + lokasi,
 * koordinat aset mengikuti koordinat lokasi pelanggan, dan kode aset mengikuti
 * kode lokasi (bila ada) supaya penamaan konsisten dengan data lapangan.
 */
export function buildInstallationQuickAssetDraftFromContext(input: {
  assetType: InstallationQuickAssetType;
  context: InstallationQuickAssetContext;
}): InstallationQuickAssetDraft {
  const { assetType, context } = input;
  const latitude = normalizeCoord(context.locationLatitude);
  const longitude = normalizeCoord(context.locationLongitude);

  return {
    ...buildDefaultInstallationQuickAssetDraft(),
    asset_type: assetType,
    name: buildInstallationQuickAssetSuggestedName({
      assetType,
      customerName: context.customerName,
      locationLabel: context.locationLabel,
    }),
    code: normalizeIdentifier(context.locationCode || ''),
    latitude: latitude == null ? '' : String(latitude),
    longitude: longitude == null ? '' : String(longitude),
  };
}

/**
 * Bila tipe aset berubah setelah draf terisi otomatis, perbarui hanya nilai yang
 * MASIH berasal dari usulan — jangan menimpa yang sudah diketik pengguna.
 */
export function syncInstallationQuickAssetDraftOnTypeChange(input: {
  draft: InstallationQuickAssetDraft;
  nextAssetType: InstallationQuickAssetType;
  context: InstallationQuickAssetContext;
}): InstallationQuickAssetDraft {
  const { draft, nextAssetType, context } = input;
  const currentSuggestedName = buildInstallationQuickAssetSuggestedName({
    assetType: draft.asset_type,
    customerName: context.customerName,
    locationLabel: context.locationLabel,
  });
  const nextSuggestedName = buildInstallationQuickAssetSuggestedName({
    assetType: nextAssetType,
    customerName: context.customerName,
    locationLabel: context.locationLabel,
  });
  const suggestedCode = normalizeIdentifier(context.locationCode || '');
  const currentCode = normalizeIdentifier(draft.code);

  return {
    ...draft,
    asset_type: nextAssetType,
    name:
      !draft.name.trim() || draft.name === currentSuggestedName ? nextSuggestedName : draft.name,
    // Kode kosong atau masih sama dengan usulan → ikut usulan baru.
    code: !currentCode || (!!suggestedCode && currentCode === suggestedCode) ? suggestedCode : draft.code,
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

/** Konversi teks input koordinat ke angka; '' → null (bukan 0). */
export function parseQuickAssetCoordinate(value: string): number | null {
  const raw = (value ?? '').trim();
  if (!raw) return null;
  const parsed = Number(raw);
  return Number.isFinite(parsed) ? parsed : null;
}

/** Validasi rentang koordinat supaya tidak tersimpan titik mustahil. */
export function validateQuickAssetCoordinates(
  latitude: string,
  longitude: string,
): string | null {
  const lat = parseQuickAssetCoordinate(latitude);
  const lng = parseQuickAssetCoordinate(longitude);
  if (lat == null && lng == null) return null; // boleh kosong
  if (lat == null || lng == null) {
    return 'Isi lintang dan bujur sekaligus, atau kosongkan keduanya.';
  }
  if (lat < -90 || lat > 90) return 'Lintang harus antara -90 dan 90.';
  if (lng < -180 || lng > 180) return 'Bujur harus antara -180 dan 180.';
  return null;
}

export function validateInstallationQuickAssetDraft(
  draft: InstallationQuickAssetDraft,
): string | null {
  if (!INSTALLATION_QUICK_ASSET_TYPES.includes(draft.asset_type)) {
    return 'Pembuatan cepat hanya mendukung ONT, ONU, atau ODP.';
  }
  if (!draft.name.trim()) {
    return 'Nama aset wajib diisi.';
  }
  if (!draft.serial_number.trim()) {
    return 'Nomor seri wajib diisi.';
  }
  return validateQuickAssetCoordinates(draft.latitude, draft.longitude);
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
    duplicates.code = 'Kode aset sudah dipakai di registri.';
  }

  if (
    normalizedSerial &&
    assets.some((asset) => normalizeIdentifier(asset.serial_number || '') === normalizedSerial)
  ) {
    duplicates.serial_number = 'Nomor seri sudah terdaftar di registri.';
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
    /* Parent dari ODP yang dipilih teknisi; null bila belum ada jalur FTTH. */
    parent_asset_id: draft.parent_asset_id || null,
    latitude: parseQuickAssetCoordinate(draft.latitude),
    longitude: parseQuickAssetCoordinate(draft.longitude),
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

/** Angka koordinat → nilai rapi; NaN/Infinity → null supaya tidak lolos ke API. */
function normalizeCoord(value: number | null | undefined): number | null {
  if (value == null) return null;
  if (!Number.isFinite(value)) return null;
  return value;
}
