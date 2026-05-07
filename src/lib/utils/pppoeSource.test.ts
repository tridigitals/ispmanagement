import { describe, expect, it } from 'vitest';

import {
  getPppoeApplyActionFallback,
  getPppoeCreateActionFallback,
  getPppoeCreatedAndAppliedToastFallback,
  getPppoeProvisioningTargetFallback,
  getPppoeSyncDisplay,
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

  it('uses native radius provisioning language for managed radius status', () => {
    expect(
      getPppoeSyncDisplay({
        account_source: 'managed_radius',
        router_present: false,
        is_provisioned: true,
        last_sync_at: null,
        provisioned_at: '2026-04-14T01:00:00Z',
        last_error: null,
        provisioning_error: null,
      }),
    ).toEqual({
      label: 'Provisioned',
      tone: 'ok',
      syncedAt: '2026-04-14T01:00:00Z',
      error: null,
    });
  });

  it('keeps router presence for router sync status', () => {
    expect(
      getPppoeSyncDisplay({
        account_source: 'router',
        router_present: false,
        is_provisioned: true,
        last_sync_at: '2026-04-14T02:00:00Z',
        provisioned_at: '2026-04-14T01:00:00Z',
        last_error: 'router missing',
        provisioning_error: null,
      }),
    ).toEqual({
      label: 'Missing',
      tone: 'warn',
      syncedAt: '2026-04-14T02:00:00Z',
      error: 'router missing',
    });
  });
});
