import { describe, expect, it } from 'vitest';

import { shouldSuppressPopupOnTargetPick } from './networkMapClickGuards';

describe('networkMapClickGuards', () => {
  it('suppresses popup when connect mode is active and the clicked marker is a target type', () => {
    expect(shouldSuppressPopupOnTargetPick(true, 'router')).toBe(true);
    expect(shouldSuppressPopupOnTargetPick(true, 'topology_asset')).toBe(true);
    expect(shouldSuppressPopupOnTargetPick(true, 'node')).toBe(true);
  });

  it('keeps popup behavior normal outside connect mode', () => {
    expect(shouldSuppressPopupOnTargetPick(false, 'router')).toBe(false);
  });
});
