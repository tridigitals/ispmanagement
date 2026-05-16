import { describe, expect, it } from 'vitest';

import { getNetworkAssetFormProfile } from './networkAssetFormProfile';

describe('networkAssetFormProfile', () => {
  it('treats ftth distribution assets as topology-first forms with optional hardware identity', () => {
    expect(getNetworkAssetFormProfile('odp')).toMatchObject({
      group: 'distribution',
      identityFields: ['type', 'status', 'name', 'code'],
      hardwareFieldsInline: [],
      hardwareFieldsOptional: ['serial_number', 'vendor', 'model'],
    });
  });

  it('treats endpoint devices as hardware-focused forms', () => {
    expect(getNetworkAssetFormProfile('ont')).toMatchObject({
      group: 'endpoint_device',
      identityFields: ['type', 'status', 'name', 'code'],
      hardwareFieldsInline: ['serial_number', 'vendor', 'model'],
      hardwareFieldsOptional: [],
    });
  });

  it('treats active network devices as hardware-focused forms', () => {
    expect(getNetworkAssetFormProfile('router')).toMatchObject({
      group: 'active_device',
      hardwareFieldsInline: ['serial_number', 'vendor', 'model'],
      detailSectionTitle: 'Device Profile & Capacity',
    });
  });
});
