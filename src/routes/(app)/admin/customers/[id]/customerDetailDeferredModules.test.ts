import { describe, expect, it } from 'vitest';

import {
  loadCustomerPppoeHelperModule,
  loadCustomerTimelineHelperModule,
} from './customerDetailDeferredModules';

describe('customer detail deferred modules', () => {
  it('loads and caches the pppoe helper module on demand', async () => {
    const first = await loadCustomerPppoeHelperModule();
    const second = await loadCustomerPppoeHelperModule();

    expect(first.pppoeToolbar).toEqual({
      showSearch: true,
      showRefresh: true,
      showCreate: false,
      showReconcile: false,
    });
    expect(typeof first.getPppoeSyncDisplay).toBe('function');
    expect(typeof first.getPppoeProvisioningTargetFallback).toBe('function');
    expect(typeof first.getPppoeApplyActionFallback).toBe('function');
    expect(second).toBe(first);
  });

  it('loads and caches the timeline helper module on demand', async () => {
    const first = await loadCustomerTimelineHelperModule();
    const second = await loadCustomerTimelineHelperModule();

    expect(typeof first.buildCustomerTimelineRows).toBe('function');
    expect(second).toBe(first);
  });
});
