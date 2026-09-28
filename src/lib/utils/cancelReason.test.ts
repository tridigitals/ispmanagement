import { describe, expect, it } from 'vitest';

import { buildDefaultInstallationCancelReason } from './cancelReason';

describe('buildDefaultInstallationCancelReason', () => {
  it('returns a non-empty default cancellation reason that satisfies minimum length', () => {
    const reason = buildDefaultInstallationCancelReason();

    expect(reason.length).toBeGreaterThanOrEqual(10);
    expect(reason).toBe('Cancelled by admin');
  });
});
