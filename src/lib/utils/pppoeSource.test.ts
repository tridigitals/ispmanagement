import { describe, expect, it } from 'vitest';

import {
  getPppoeApplyActionFallback,
  getPppoeCreateActionFallback,
  getPppoeCreatedAndAppliedToastFallback,
  getPppoeProvisioningTargetFallback,
} from './pppoeSource';

describe('pppoe source helpers', () => {
  it('returns router-specific fallback labels', () => {
    expect(getPppoeProvisioningTargetFallback('router')).toBe('router');
    expect(getPppoeApplyActionFallback('router')).toBe('Apply to router');
    expect(getPppoeCreateActionFallback('router')).toBe('Create & apply to router');
    expect(getPppoeCreatedAndAppliedToastFallback('router')).toBe(
      'PPPoE account created and applied to router',
    );
  });

  it('returns radius-specific fallback labels', () => {
    expect(getPppoeProvisioningTargetFallback('managed_radius')).toBe('RADIUS');
    expect(getPppoeApplyActionFallback('managed_radius')).toBe('Apply to RADIUS');
    expect(getPppoeCreateActionFallback('managed_radius')).toBe('Create & apply to RADIUS');
    expect(getPppoeCreatedAndAppliedToastFallback('managed_radius')).toBe(
      'PPPoE account created and applied to RADIUS',
    );
  });
});
