import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  detailsModal: { name: 'user-details-modal' },
  actionModals: { name: 'user-action-modals' },
}));

vi.mock('$lib/components/superadmin/users/UserDetailsModal.svelte', () => ({
  default: sentinels.detailsModal,
}));

vi.mock('$lib/components/superadmin/users/UserActionModals.svelte', () => ({
  default: sentinels.actionModals,
}));

import { loadSuperadminUsersModalModules } from './usersPageModules';

describe('superadmin users page modules', () => {
  it('loads and caches modal modules lazily', async () => {
    const first = await loadSuperadminUsersModalModules();
    const second = await loadSuperadminUsersModalModules();

    expect(first.UserDetailsModalComponent).toBe(sentinels.detailsModal);
    expect(first.UserActionModalsComponent).toBe(sentinels.actionModals);
    expect(second).toBe(first);
  });
});
