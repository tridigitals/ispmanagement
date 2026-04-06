import { describe, expect, it } from 'vitest';

import {
  getPppoeAssignmentPayload,
  getPppoeAssignmentPreview,
} from './pppoePackageAssignment';

describe('pppoe package assignment helpers', () => {
  it('derives payload from selected package mapping instead of legacy account fields', () => {
    const mappings = [
      {
        package_id: 'pkg-1',
        package_name: 'Basic 10M',
        router_profile_name: 'basic-10m',
        address_pool: 'pool-basic-10m',
      },
    ];

    expect(
      getPppoeAssignmentPayload({
        packageId: 'pkg-1',
        mappings,
        current: {
          router_profile_name: 'legacy-profile',
          remote_address: '10.10.10.10',
          address_pool: 'legacy-pool',
        },
      }),
    ).toEqual({
      source: 'package',
      hasPackageMapping: true,
      router_profile_name: 'basic-10m',
      remote_address: null,
      address_pool: 'pool-basic-10m',
    });
  });

  it('falls back to current account fields when no package mapping is available', () => {
    expect(
      getPppoeAssignmentPayload({
        packageId: '',
        mappings: [],
        current: {
          router_profile_name: 'legacy-profile',
          remote_address: '10.10.10.10',
          address_pool: 'legacy-pool',
        },
      }),
    ).toEqual({
      source: 'account',
      hasPackageMapping: false,
      router_profile_name: 'legacy-profile',
      remote_address: '10.10.10.10',
      address_pool: 'legacy-pool',
    });
  });

  it('builds a read-only preview using package mapping and pool as effective remote value', () => {
    const preview = getPppoeAssignmentPreview({
      packageId: 'pkg-1',
      mappings: [
        {
          package_id: 'pkg-1',
          package_name: 'Basic 10M',
          router_profile_name: 'basic-10m',
          address_pool: 'pool-basic-10m',
        },
      ],
      current: {},
    });

    expect(preview).toEqual({
      source: 'package',
      hasPackageMapping: true,
      profileName: 'basic-10m',
      remoteAddress: 'pool-basic-10m',
      addressPool: 'pool-basic-10m',
    });
  });
});
