import { describe, expect, it } from 'vitest';

import {
  buildNetworkAssetMetadata,
  createNetworkAssetDetailDraft,
  getNetworkAssetDetailFields,
  getNetworkAssetDetailSummary,
  validateNetworkAssetDetailDraft,
} from './networkAssetDetails';

describe('network asset detail helpers', () => {
  it('returns grouped detail fields for supported asset types', () => {
    expect(getNetworkAssetDetailFields('ont').map((field) => field.key)).toEqual([
      'pon_serial',
      'loid',
      'mac_address',
    ]);
    expect(getNetworkAssetDetailFields('switch').map((field) => field.key)).toEqual([
      'management_ip',
      'uplink_port',
      'firmware_version',
    ]);
  });

  it('creates a detail draft from metadata using only known fields', () => {
    expect(
      createNetworkAssetDetailDraft('odp', {
        total_port_capacity: '8',
        splitter_ratio: '1:8',
        ignored_key: 'skip',
      }),
    ).toEqual({
      total_port_capacity: '8',
      splitter_ratio: '1:8',
      coverage_area: '',
    });
  });

  it('merges typed metadata while removing emptied values', () => {
    expect(
      buildNetworkAssetMetadata(
        'router',
        {
          management_ip: '10.0.0.1',
          uplink_port: '',
          firmware_version: 'RouterOS 7.15',
        },
        {
          legacy_note: 'keep',
          uplink_port: 'ether1',
        },
      ),
    ).toEqual({
      legacy_note: 'keep',
      management_ip: '10.0.0.1',
      firmware_version: 'RouterOS 7.15',
    });
  });

  it('builds short human-readable summaries from metadata', () => {
    expect(
      getNetworkAssetDetailSummary({
        asset_type: 'ups',
        metadata: {
          battery_capacity_ah: '100',
          backup_runtime_minutes: '120',
          power_phase: '1 phase',
        },
      } as any),
    ).toEqual([
      'Battery Capacity (Ah): 100',
      'Backup Runtime (Minutes): 120',
      'Power Phase: 1 phase',
    ]);
  });

  it('validates ip, mac, and positive integer detail fields', () => {
    expect(
      validateNetworkAssetDetailDraft('router', {
        management_ip: '10.0.0.256',
        uplink_port: 'ether1',
        firmware_version: '7.15',
      }),
    ).toEqual(['Management IP: must be a valid IP address']);

    expect(
      validateNetworkAssetDetailDraft('ont', {
        pon_serial: 'ZTE123',
        loid: 'CUST-1',
        mac_address: 'AABBCCDDEEFF',
      }),
    ).toEqual(['MAC Address: must use MAC format like AA:BB:CC:DD:EE:FF']);

    expect(
      validateNetworkAssetDetailDraft('ups', {
        battery_capacity_ah: '0',
        backup_runtime_minutes: '120',
        power_phase: '1 phase',
      }),
    ).toEqual(['Battery Capacity (Ah): must be a positive whole number']);

    expect(
      validateNetworkAssetDetailDraft('odp', {
        total_port_capacity: '0',
        splitter_ratio: '1:8',
        coverage_area: 'Cluster A',
      }),
    ).toEqual(['Total Port Capacity: must be a positive whole number']);
  });
});
