import { describe, expect, it } from 'vitest';

import {
  applyInstallationQuickAssetInputChange,
  buildDefaultInstallationQuickAssetDraft,
  buildInstallationQuickAssetDraftFromContext,
  findInstallationQuickAssetDuplicates,
  buildInstallationQuickAssetSuggestedName,
  buildInstallationQuickAssetPayload,
  parseQuickAssetCoordinate,
  syncInstallationQuickAssetDraftOnTypeChange,
  validateInstallationQuickAssetDraft,
  validateQuickAssetCoordinates,
  type InstallationQuickAssetDraft,
} from './installationQuickAsset';

/** Draf lengkap: helper supaya test tetap terbaca walau field bertambah. */
function draft(overrides: Partial<InstallationQuickAssetDraft> = {}): InstallationQuickAssetDraft {
  return { ...buildDefaultInstallationQuickAssetDraft(), ...overrides };
}

describe('installationQuickAsset', () => {
  it('builds a default quick-create draft', () => {
    expect(buildDefaultInstallationQuickAssetDraft()).toEqual({
      asset_type: 'ont',
      name: '',
      code: '',
      vendor: '',
      model: '',
      serial_number: '',
      latitude: '',
      longitude: '',
      parent_asset_id: '',
    });
  });

  it('suggests a name from customer and location context', () => {
    expect(
      buildInstallationQuickAssetSuggestedName({
        assetType: 'ont',
        customerName: 'Alpha Net',
        locationLabel: 'Rumah Utama',
      }),
    ).toBe('ONT Alpha Net - Rumah Utama');

    expect(
      buildInstallationQuickAssetSuggestedName({
        assetType: 'onu',
        customerName: 'Alpha Net',
        locationLabel: '',
      }),
    ).toBe('ONU Alpha Net');
  });
});

/**
 * Inti permintaan: "bisa create dari situ dan detail-detailnya bisa ambil dari
 * data instalasi". Draf harus terisi otomatis dari work order — bukan kosong.
 */
describe('mengisi draf dari data instalasi', () => {
  it('mengambil nama, koordinat lokasi, dan kode dari data work order', () => {
    const hasil = buildInstallationQuickAssetDraftFromContext({
      assetType: 'ont',
      context: {
        customerName: 'Budi Santoso',
        locationLabel: 'kebondalem',
        locationLatitude: -7.235423,
        locationLongitude: 110.41976,
        locationCode: 'kebondalem',
      },
    });

    expect(hasil.name).toBe('ONT Budi Santoso - kebondalem');
    // Inilah nilai yang dulu harus diketik manual padahal backend sudah punya.
    expect(hasil.latitude).toBe('-7.235423');
    expect(hasil.longitude).toBe('110.41976');
    expect(hasil.code).toBe('KEBONDALEM');
  });

  it('membiarkan koordinat kosong bila instalasi belum punya lokasi terpetakan', () => {
    const hasil = buildInstallationQuickAssetDraftFromContext({
      assetType: 'ont',
      context: { customerName: 'Tanpa Lokasi', locationLatitude: null, locationLongitude: null },
    });

    expect(hasil.latitude).toBe('');
    expect(hasil.longitude).toBe('');
    expect(hasil.name).toBe('ONT Tanpa Lokasi');
  });

  it('mengabaikan koordinat tidak valid (NaN/Infinity) alih-alih mengirim 0', () => {
    const hasil = buildInstallationQuickAssetDraftFromContext({
      assetType: 'ont',
      context: { locationLatitude: Number.NaN, locationLongitude: Number.POSITIVE_INFINITY },
    });

    expect(hasil.latitude).toBe('');
    expect(hasil.longitude).toBe('');
  });

  it('memperbarui nama & kode saat tipe berubah, tetapi tidak menimpa isian pengguna', () => {
    const context = { customerName: 'Budi', locationLabel: 'kebondalem', locationCode: 'kebondalem' };

    // Nama & kode masih usulan → ikut berubah.
    const ikut = syncInstallationQuickAssetDraftOnTypeChange({
      draft: draft({ name: 'ONT Budi - kebondalem', code: 'KEBONDALEM' }),
      nextAssetType: 'odp',
      context,
    });
    expect(ikut.asset_type).toBe('odp');
    expect(ikut.name).toBe('ODP Budi - kebondalem');
    expect(ikut.code).toBe('KEBONDALEM');

    // Sudah diketik sendiri → dipertahankan.
    const dipertahankan = syncInstallationQuickAssetDraftOnTypeChange({
      draft: draft({ name: 'ONT VIP Customer', code: 'KODE-SAYA' }),
      nextAssetType: 'odp',
      context,
    });
    expect(dipertahankan.name).toBe('ONT VIP Customer');
    expect(dipertahankan.code).toBe('KODE-SAYA');
  });
});

describe('mendukung ODP, bukan hanya ONT/ONU', () => {
  it('menerima odp sebagai tipe yang bisa dibuat dari instalasi', () => {
    expect(
      validateInstallationQuickAssetDraft(
        draft({ asset_type: 'odp', name: 'ODP Kebondalem', serial_number: 'ODP-1' }),
      ),
    ).toBeNull();
  });

  it('membuat payload ODP dengan tipe yang benar', () => {
    const payload = buildInstallationQuickAssetPayload({
      draft: draft({ asset_type: 'odp', name: 'ODP Kebondalem', serial_number: 'odp 1' }),
      customer_id: 'cust-1',
      location_id: 'loc-1',
      work_order_id: 'wo-1',
    });
    expect(payload.asset_type).toBe('odp');
    expect(payload.serial_number).toBe('ODP-1');
  });

  it('menolak tipe di luar ONT/ONU/ODP', () => {
    expect(
      validateInstallationQuickAssetDraft(
        draft({ asset_type: 'olt' as any, name: 'X', serial_number: '1' }),
      ),
    ).toBe('Pembuatan cepat hanya mendukung ONT, ONU, atau ODP.');
  });
});

describe('koordinat', () => {
  it('parseQuickAssetCoordinate: kosong -> null, bukan 0', () => {
    expect(parseQuickAssetCoordinate('')).toBeNull();
    expect(parseQuickAssetCoordinate('   ')).toBeNull();
    expect(parseQuickAssetCoordinate('abc')).toBeNull();
    expect(parseQuickAssetCoordinate('-6.2')).toBe(-6.2);
    expect(parseQuickAssetCoordinate('0')).toBe(0);
  });

  it('menolak lintang/bujur yang tidak masuk akal', () => {
    expect(validateQuickAssetCoordinates('', '')).toBeNull();
    expect(validateQuickAssetCoordinates('-7.2', '')).toMatch(/sekaligus/);
    expect(validateQuickAssetCoordinates('95', '110')).toMatch(/Lintang/);
    expect(validateQuickAssetCoordinates('-7', '200')).toMatch(/Bujur/);
    expect(validateQuickAssetCoordinates('-7.2', '110.4')).toBeNull();
  });

  it('validasi ikut menangkap koordinat salah sebelum submit', () => {
    expect(
      validateInstallationQuickAssetDraft(
        draft({ name: 'ONT A', serial_number: 'SN-1', latitude: '95', longitude: '110' }),
      ),
    ).toMatch(/Lintang/);
  });
});

describe('validasi & payload', () => {
  it('validates minimal required quick-create fields', () => {
    expect(validateInstallationQuickAssetDraft(draft({ name: '' }))).toBe('Nama aset wajib diisi.');
    expect(validateInstallationQuickAssetDraft(draft({ name: 'ONT ZTE F670L' }))).toBe(
      'Nomor seri wajib diisi.',
    );
  });

  it('builds stable create payload for installation-created assets', () => {
    expect(
      buildInstallationQuickAssetPayload({
        draft: draft({
          asset_type: 'ont',
          name: ' ONT Customer A ',
          code: ' ONT-001 ',
          vendor: ' ZTE ',
          model: ' F670L ',
          serial_number: ' SN-123 ',
        }),
        customer_id: 'cust-1',
        location_id: 'loc-1',
        work_order_id: 'wo-1',
        notes: ' Created from installation ',
      }),
    ).toEqual({
      asset_type: 'ont',
      name: 'ONT Customer A',
      code: 'ONT-001',
      vendor: 'ZTE',
      model: 'F670L',
      serial_number: 'SN-123',
      status: 'available',
      customer_id: 'cust-1',
      location_id: 'loc-1',
      work_order_id: 'wo-1',
      parent_asset_id: null,
      latitude: null,
      longitude: null,
      notes: 'Created from installation',
      metadata: {},
    });
  });

  it('mengirim koordinat dan induk ODP pada payload', () => {
    const payload = buildInstallationQuickAssetPayload({
      draft: draft({
        name: 'ONT A',
        serial_number: 'SN-1',
        latitude: '-7.235423',
        longitude: '110.41976',
        parent_asset_id: 'odp-1',
      }),
      customer_id: 'cust-1',
      location_id: 'loc-1',
      work_order_id: 'wo-1',
    });

    expect(payload.latitude).toBe(-7.235423);
    expect(payload.longitude).toBe(110.41976);
    expect(payload.parent_asset_id).toBe('odp-1');
  });

  it('normalizes code, serial, vendor, and model input changes', () => {
    expect(applyInstallationQuickAssetInputChange('code', ' ont 001 /a ')).toBe('ONT-001-A');
    expect(applyInstallationQuickAssetInputChange('serial_number', ' sn 123 / zte ')).toBe(
      'SN-123-ZTE',
    );
    expect(applyInstallationQuickAssetInputChange('vendor', ' zte   corp ')).toBe('ZTE CORP');
    expect(applyInstallationQuickAssetInputChange('model', ' f670l   v2 ')).toBe('F670L V2');
    expect(applyInstallationQuickAssetInputChange('name', '  Ont Customer A  ')).toBe(
      'Ont Customer A',
    );
  });

  it('detects duplicate serial and code from existing tenant asset registry', () => {
    expect(
      findInstallationQuickAssetDuplicates(
        draft({ name: 'ONT Alpha', code: 'ont 001', serial_number: 'sn 123' }),
        [
          { code: 'ONT-001', serial_number: 'SN-123' },
          { code: 'ONU-999', serial_number: 'ONU-999' },
        ] as any,
      ),
    ).toEqual({
      code: 'Kode aset sudah dipakai di registri.',
      serial_number: 'Nomor seri sudah terdaftar di registri.',
    });
  });
});
