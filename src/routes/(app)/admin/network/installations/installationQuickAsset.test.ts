import { describe, expect, it } from 'vitest';

import {
  applyInstallationQuickAssetInputChange,
  buildDefaultInstallationQuickAssetDraft,
  findInstallationQuickAssetDuplicates,
  buildInstallationQuickAssetSuggestedName,
  buildInstallationQuickAssetPayload,
  syncInstallationQuickAssetDraftOnTypeChange,
  validateInstallationQuickAssetDraft,
} from './installationQuickAsset';

describe('installationQuickAsset', () => {
  it('builds a default quick-create draft for terminal assets', () => {
    expect(buildDefaultInstallationQuickAssetDraft()).toEqual({
      asset_type: 'ont',
      name: '',
      code: '',
      vendor: '',
      model: '',
      serial_number: '',
    });
  });

  it('suggests a terminal asset name from customer and location context', () => {
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

  it('updates suggested name when quick-create asset type changes', () => {
    expect(
      syncInstallationQuickAssetDraftOnTypeChange({
        draft: {
          asset_type: 'ont',
          name: 'ONT Alpha Net - Rumah Utama',
          code: '',
          vendor: '',
          model: '',
          serial_number: '',
        },
        nextAssetType: 'onu',
        customerName: 'Alpha Net',
        locationLabel: 'Rumah Utama',
      }),
    ).toEqual({
      asset_type: 'onu',
      name: 'ONU Alpha Net - Rumah Utama',
      code: '',
      vendor: '',
      model: '',
      serial_number: '',
    });
  });

  it('preserves custom name when quick-create asset type changes', () => {
    expect(
      syncInstallationQuickAssetDraftOnTypeChange({
        draft: {
          asset_type: 'ont',
          name: 'ONT VIP Customer',
          code: '',
          vendor: '',
          model: '',
          serial_number: '',
        },
        nextAssetType: 'onu',
        customerName: 'Alpha Net',
        locationLabel: 'Rumah Utama',
      }),
    ).toEqual({
      asset_type: 'onu',
      name: 'ONT VIP Customer',
      code: '',
      vendor: '',
      model: '',
      serial_number: '',
    });
  });

  it('validates minimal required quick-create fields', () => {
    expect(
      validateInstallationQuickAssetDraft({
        asset_type: 'onu',
        name: '',
        code: '',
        vendor: '',
        model: '',
        serial_number: '',
      }),
    ).toBe('Asset name is required.');

    expect(
      validateInstallationQuickAssetDraft({
        asset_type: 'ont',
        name: 'ONT ZTE F670L',
        code: '',
        vendor: '',
        model: '',
        serial_number: '',
      }),
    ).toBe('Serial number is required.');
  });

  it('builds stable create payload for installation-created assets', () => {
    expect(
      buildInstallationQuickAssetPayload({
        draft: {
          asset_type: 'ont',
          name: ' ONT Customer A ',
          code: ' ONT-001 ',
          vendor: ' ZTE ',
          model: ' F670L ',
          serial_number: ' SN-123 ',
        },
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
      notes: 'Created from installation',
      metadata: {},
    });
  });

  it('normalizes code, serial, vendor, and model input changes for quick create', () => {
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
        {
          asset_type: 'ont',
          name: 'ONT Alpha',
          code: 'ont 001',
          vendor: 'ZTE',
          model: 'F670L',
          serial_number: 'sn 123',
        },
        [
          {
            code: 'ONT-001',
            serial_number: 'SN-123',
          },
          {
            code: 'ONU-999',
            serial_number: 'ONU-999',
          },
        ] as any,
      ),
    ).toEqual({
      code: 'Asset code already exists in registry.',
      serial_number: 'Serial number already exists in registry.',
    });
  });
});
