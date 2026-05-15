import { describe, expect, it } from 'vitest';

import {
  getNetworkAssetGroup,
  getNetworkAssetGroupLabel,
  NETWORK_ASSET_STATUSES,
  NETWORK_ASSET_TYPES,
  getDefaultNetworkAssetStatus,
  getNetworkAssetStatusLabel,
  getNetworkAssetTypeLabel,
} from './networkAssetTypes';

describe('network asset type helpers', () => {
  it('exposes the supported FTTH and infrastructure asset types', () => {
    expect(NETWORK_ASSET_TYPES).toEqual([
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
    ]);
  });

  it('exposes the supported Sprint 1 statuses', () => {
    expect(NETWORK_ASSET_STATUSES).toEqual([
      'available',
      'reserved',
      'installed',
      'faulty',
      'retired',
    ]);
  });

  it('maps known labels and falls back for unknown values', () => {
    expect(getNetworkAssetTypeLabel('olt')).toBe('OLT');
    expect(getNetworkAssetTypeLabel('splitter')).toBe('Splitter');
    expect(getNetworkAssetTypeLabel('media_converter')).toBe('Media Converter');
    expect(getNetworkAssetStatusLabel('faulty')).toBe('Faulty');
    expect(getNetworkAssetStatusLabel('custom')).toBe('custom');
  });

  it('derives the asset group from the asset type', () => {
    expect(getNetworkAssetGroup('odp')).toBe('access_fiber');
    expect(getNetworkAssetGroup('switch')).toBe('infrastructure');
    expect(getNetworkAssetGroupLabel('infrastructure')).toBe('Infrastructure');
  });

  it('defaults new assets to available status', () => {
    expect(getDefaultNetworkAssetStatus()).toBe('available');
  });
});
