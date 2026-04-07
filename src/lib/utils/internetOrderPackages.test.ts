import { describe, expect, it } from 'vitest';

import type { IspPackage } from '$lib/api/types';

import { getVisibleInternetOrderPackages } from './internetOrderPackages';

function createPackage(id: string, name: string): IspPackage {
  return {
    id,
    tenant_id: 'tenant-1',
    service_type: 'internet',
    name,
    description: null,
    features: [],
    is_active: true,
    price_monthly: 100000,
    price_yearly: 1000000,
    created_at: '2026-04-06T00:00:00Z',
    updated_at: '2026-04-06T00:00:00Z',
  };
}

describe('internet order package helpers', () => {
  it('keeps all active catalog packages visible without coverage filtering', () => {
    const packages = [
      createPackage('pkg-1', 'Starter'),
      createPackage('pkg-2', 'Pro'),
      createPackage('pkg-3', 'Business'),
    ];

    expect(getVisibleInternetOrderPackages(packages)).toEqual(packages);
  });
});
