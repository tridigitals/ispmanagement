import { describe, expect, it } from 'vitest';

import {
  getPackageRouterMappingErrorFallback,
  getPackageRouterMappingReferenceError,
} from './packageRouterMapping';

describe('package router mapping helpers', () => {
  it('returns a profile validation error when the selected PPP profile is no longer available', () => {
    expect(
      getPackageRouterMappingReferenceError({
        routerId: 'router-1',
        profileName: 'basic-10m',
        profileSuggestions: [{ id: '1', name: 'starter' }],
        poolName: '',
        poolSuggestions: [],
      }),
    ).toBe(
      "Selected PPP profile 'basic-10m' is no longer available on this router. Sync PPP profiles and choose a valid profile.",
    );
  });

  it('returns a pool validation error when the selected IP pool is no longer available', () => {
    expect(
      getPackageRouterMappingReferenceError({
        routerId: 'router-1',
        profileName: 'starter',
        profileSuggestions: [{ id: '1', name: 'starter' }],
        poolName: 'pool-old',
        poolSuggestions: [{ id: '1', name: 'pool-new' }],
      }),
    ).toBe(
      "Selected IP pool 'pool-old' is no longer available on this router. Sync IP pools and choose a valid pool.",
    );
  });

  it('returns null when the selected mapping references still exist', () => {
    expect(
      getPackageRouterMappingReferenceError({
        routerId: 'router-1',
        profileName: 'starter',
        profileSuggestions: [{ id: '1', name: 'starter' }],
        poolName: 'pool-new',
        poolSuggestions: [{ id: '1', name: 'pool-new' }],
      }),
    ).toBeNull();
  });

  it('formats backend stale-reference messages into friendly copy', () => {
    expect(
      getPackageRouterMappingErrorFallback(
        'Selected PPP profile does not exist on this router. Sync PPP profiles and choose a valid profile.',
      ),
    ).toBe(
      'The selected PPP profile is no longer available on this router. Sync PPP profiles and choose another profile.',
    );

    expect(
      getPackageRouterMappingErrorFallback(
        "Selected IP pool 'pool-old' does not exist on this router. Sync IP pools and choose a valid pool.",
      ),
    ).toBe(
      'The selected IP pool is no longer available on this router. Sync IP pools and choose another pool.',
    );
  });
});
