import { describe, expect, it } from 'vitest';

import {
  copyCustomerLocationCoordinates,
  formatNetworkAssetCoordinates,
  parseNetworkAssetCoordinates,
} from './networkAssetCoordinates';

describe('networkAssetCoordinates', () => {
  it('parses empty coordinate pairs as null', () => {
    expect(parseNetworkAssetCoordinates('', '')).toEqual({
      latitude: null,
      longitude: null,
      error: null,
    });
  });

  it('requires latitude and longitude together', () => {
    expect(parseNetworkAssetCoordinates('-6.2', '')).toEqual({
      latitude: null,
      longitude: null,
      error: 'pair',
    });
  });

  it('validates coordinate ranges', () => {
    expect(parseNetworkAssetCoordinates('-91', '106.8').error).toBe('latitude_range');
    expect(parseNetworkAssetCoordinates('-6.2', '181').error).toBe('longitude_range');
  });

  it('copies customer location coordinates into string draft fields', () => {
    expect(
      copyCustomerLocationCoordinates({
        latitude: -6.21462,
        longitude: 106.84513,
      }),
    ).toEqual({
      latitude: '-6.21462',
      longitude: '106.84513',
    });
  });

  it('formats coordinate labels lightly for registry display', () => {
    expect(formatNetworkAssetCoordinates(-6.21462, 106.84513)).toBe('-6.214620, 106.845130');
    expect(formatNetworkAssetCoordinates(null, 106.84513)).toBe('');
  });
});
