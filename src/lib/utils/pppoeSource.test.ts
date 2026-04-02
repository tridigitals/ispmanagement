import { describe, expect, it } from 'vitest';

import { getPppoeApplyActionFallback, getPppoeProvisioningTargetFallback } from './pppoeSource';

describe('pppoe source helpers', () => {
  it('returns router-specific fallback labels', () => {
    expect(getPppoeProvisioningTargetFallback('router')).toBe('router');
    expect(getPppoeApplyActionFallback('router')).toBe('Apply to router');
  });

  it('returns radius-specific fallback labels', () => {
    expect(getPppoeProvisioningTargetFallback('managed_radius')).toBe('RADIUS');
    expect(getPppoeApplyActionFallback('managed_radius')).toBe('Apply to RADIUS');
  });
});
