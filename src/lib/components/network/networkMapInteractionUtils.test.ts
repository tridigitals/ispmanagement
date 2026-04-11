import { describe, expect, it } from 'vitest';

import {
  computePopupPlacement,
  computePopupViewportNudge,
} from './networkMapInteractionUtils';

describe('computePopupViewportNudge', () => {
  it('returns zero offset when popup is already inside map viewport', () => {
    const result = computePopupViewportNudge({
      popupRect: { left: 120, right: 320, top: 120, bottom: 260 },
      mapRect: { left: 40, right: 420, top: 40, bottom: 340 },
      padding: 16,
    });

    expect(result).toEqual({ x: 0, y: 0 });
  });

  it('nudges popup back inside viewport when it overflows right and bottom edges', () => {
    const result = computePopupViewportNudge({
      popupRect: { left: 190, right: 430, top: 140, bottom: 360 },
      mapRect: { left: 40, right: 420, top: 40, bottom: 340 },
      padding: 16,
    });

    expect(result).toEqual({ x: -26, y: -36 });
  });
});

describe('computePopupPlacement', () => {
  it('prefers a left-side anchor when the point is near the right edge', () => {
    const result = computePopupPlacement({
      point: { x: 360, y: 180 },
      mapSize: { width: 400, height: 320 },
      popupSize: { width: 280, height: 220 },
      padding: 16,
    });

    expect(result.anchor).toBe('left');
    expect(result.offset).toBe(14);
  });

  it('prefers a bottom anchor when the point is near the top edge', () => {
    const result = computePopupPlacement({
      point: { x: 180, y: 28 },
      mapSize: { width: 400, height: 320 },
      popupSize: { width: 240, height: 180 },
      padding: 16,
    });

    expect(result.anchor).toBe('bottom');
    expect(result.offset).toBe(14);
  });
});
