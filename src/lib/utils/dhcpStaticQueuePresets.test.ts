import { describe, expect, it } from 'vitest';
import {
  buildDhcpStaticQueueRateLimitPresets,
  extractDhcpStaticPackageBandwidthMbps,
} from './dhcpStaticQueuePresets';

describe('dhcpStaticQueuePresets', () => {
  it('extracts Mbps candidate from package name', () => {
    expect(
      extractDhcpStaticPackageBandwidthMbps({
        name: 'Internet Gold 50 Mbps',
        description: null,
        features: [],
      }),
    ).toBe(50);
  });

  it('extracts Mbps candidate from package features when name has none', () => {
    expect(
      extractDhcpStaticPackageBandwidthMbps({
        name: 'Internet Gold',
        description: null,
        features: ['Bandwidth: 30 Mbps', 'Static DHCP'],
      }),
    ).toBe(30);
  });

  it('builds queue presets with package bandwidth first', () => {
    expect(
      buildDhcpStaticQueueRateLimitPresets({
        name: 'Starter 20 Mbps',
        description: null,
        features: [],
      }),
    ).toEqual(['20M/20M', '10M/10M', '30M/30M', '50M/50M', '100M/100M']);
  });

  it('falls back to defaults when package bandwidth is unavailable', () => {
    expect(
      buildDhcpStaticQueueRateLimitPresets({
        name: 'Corporate Internet',
        description: null,
        features: ['Priority support'],
      }),
    ).toEqual(['10M/10M', '20M/20M', '30M/30M', '50M/50M', '100M/100M']);
  });
});
