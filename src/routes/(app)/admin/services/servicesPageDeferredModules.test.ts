import { describe, expect, it } from 'vitest';

import { loadServicesRouterMappingHelpers } from './servicesPageDeferredModules';

describe('services page deferred modules', () => {
  it('loads and caches router mapping helpers lazily', async () => {
    const first = await loadServicesRouterMappingHelpers();
    const second = await loadServicesRouterMappingHelpers();

    expect(typeof first.getAvailableRouterNameSuggestions).toBe('function');
    expect(typeof first.getPackageRouterMappingReferenceError).toBe('function');
    expect(typeof first.getPackageRouterMappingErrorFallback).toBe('function');
    expect(second).toBe(first);
  });
});
