import { describe, expect, it } from 'vitest';

import { shouldReuseSettingsBootstrap } from './settings';

describe('settings bootstrap cache', () => {
  it('reuses recent settings loads within the ttl window', () => {
    expect(shouldReuseSettingsBootstrap({ lastLoadedAt: 10_000, now: 50_000, ttlMs: 60_000 })).toBe(
      true,
    );
  });

  it('forces a refresh when there is no previous load or ttl expired', () => {
    expect(shouldReuseSettingsBootstrap({ lastLoadedAt: 0, now: 50_000, ttlMs: 60_000 })).toBe(
      false,
    );
    expect(
      shouldReuseSettingsBootstrap({ lastLoadedAt: 10_000, now: 80_001, ttlMs: 60_000 }),
    ).toBe(false);
  });
});
