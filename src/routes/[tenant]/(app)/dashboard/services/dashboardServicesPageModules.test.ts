import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  trackerModal: { name: 'dashboard-services-tracker-modal' },
}));

vi.mock('./DashboardServicesTrackerModal.svelte', () => ({
  default: sentinels.trackerModal,
}));

import { loadDashboardServicesTrackerModal } from './dashboardServicesPageModules';

describe('dashboard services page modules', () => {
  it('loads and caches the tracker modal lazily', async () => {
    const first = await loadDashboardServicesTrackerModal();
    const second = await loadDashboardServicesTrackerModal();

    expect(first.TrackerModalComponent).toBe(sentinels.trackerModal);
    expect(second).toBe(first);
  });
});
