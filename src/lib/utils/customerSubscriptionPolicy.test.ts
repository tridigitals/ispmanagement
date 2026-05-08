import { describe, expect, it } from 'vitest';

import {
  buildCustomerSubscriptionPolicySummary,
  clampFixedSuspendDay,
} from './customerSubscriptionPolicy';

describe('customer subscription policy helpers', () => {
  it('clamps fixed day to a safe monthly range', () => {
    expect(clampFixedSuspendDay(0)).toBe(1);
    expect(clampFixedSuspendDay(20)).toBe(20);
    expect(clampFixedSuspendDay(31)).toBe(28);
  });

  it('computes grace-period suspend preview from active-until date', () => {
    const summary = buildCustomerSubscriptionPolicySummary({
      endsAt: '2026-05-08T12:00:00Z',
      policy: {
        enabled: true,
        mode: 'grace_period',
        graceDays: 3,
        fixedDay: 1,
      },
    });

    expect(summary.activeUntilIso).toBe('2026-05-08');
    expect(summary.policyLabel).toBe('Grace 3 hari setelah masa aktif');
    expect(summary.estimatedSuspendIso).toBe('2026-05-11');
    expect(summary.estimatedSuspendMissingReason).toBeNull();
  });

  it('computes fixed-day preview in the same month when active-until is before the target day', () => {
    const summary = buildCustomerSubscriptionPolicySummary({
      endsAt: '2026-05-08',
      policy: {
        enabled: true,
        mode: 'fixed_day',
        graceDays: 3,
        fixedDay: 20,
      },
    });

    expect(summary.policyLabel).toBe('Suspend tanggal 20 setiap bulan');
    expect(summary.estimatedSuspendIso).toBe('2026-05-20');
  });

  it('computes fixed-day preview in the next month when active-until is after the target day', () => {
    const summary = buildCustomerSubscriptionPolicySummary({
      endsAt: '2026-05-21',
      policy: {
        enabled: true,
        mode: 'fixed_day',
        graceDays: 3,
        fixedDay: 20,
      },
    });

    expect(summary.estimatedSuspendIso).toBe('2026-06-20');
  });

  it('returns the missing-active-until fallback when lifecycle date is absent', () => {
    const summary = buildCustomerSubscriptionPolicySummary({
      endsAt: null,
      policy: {
        enabled: true,
        mode: 'grace_period',
        graceDays: 3,
        fixedDay: 20,
      },
    });

    expect(summary.activeUntilMissing).toBe(true);
    expect(summary.activeUntilIso).toBeNull();
    expect(summary.estimatedSuspendIso).toBeNull();
    expect(summary.estimatedSuspendMissingReason).toBe('Suspend otomatis tidak bisa dihitung');
  });
});
