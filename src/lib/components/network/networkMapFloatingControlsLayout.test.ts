import { describe, expect, it } from 'vitest';

import { getNetworkMapFloatingControlsLayout } from './networkMapFloatingControlsLayout';

describe('network map floating controls layout', () => {
  it('returns ultra-compact sizing tokens for the clean map controls card', () => {
    expect(getNetworkMapFloatingControlsLayout()).toEqual({
      desktopWidth: '244px',
      desktopPadding: '10px',
      desktopRadius: '16px',
      desktopGap: '6px',
      chipMinHeight: '28px',
      chipPaddingX: '8px',
      chipPaddingY: '5px',
      mobilePadding: '9px',
      mobileRadius: '14px',
    });
  });
});
