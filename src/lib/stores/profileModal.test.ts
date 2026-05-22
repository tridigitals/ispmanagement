import { describe, expect, it } from 'vitest';
import { get } from 'svelte/store';

import {
  closeProfileModal,
  openProfileModal,
  profileModal,
  resetProfileModal,
  setProfileModalLock,
} from './profileModal';

describe('profile modal store', () => {
  it('opens with a requested tab', () => {
    resetProfileModal();

    openProfileModal({ tab: 'security' });

    expect(get(profileModal)).toMatchObject({
      open: true,
      tab: 'security',
      locked: false,
    });
  });

  it('prevents closing while locked', () => {
    resetProfileModal();
    openProfileModal({ tab: 'security', locked: true, reason: '2fa_required' });

    closeProfileModal();

    expect(get(profileModal)).toMatchObject({
      open: true,
      tab: 'security',
      locked: true,
      reason: '2fa_required',
    });
  });

  it('can close after the lock is released', () => {
    resetProfileModal();
    openProfileModal({ tab: 'security', locked: true, reason: '2fa_required' });
    setProfileModalLock(false);

    closeProfileModal();

    expect(get(profileModal)).toMatchObject({
      open: false,
      tab: 'general',
      locked: false,
      reason: null,
    });
  });
});
