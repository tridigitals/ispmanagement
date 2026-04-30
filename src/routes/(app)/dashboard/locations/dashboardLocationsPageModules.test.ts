import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  locationFormModal: { name: 'location-form-modal' },
}));

vi.mock('./LocationFormModal.svelte', () => ({
  default: sentinels.locationFormModal,
}));

import { loadLocationFormModal } from './dashboardLocationsPageModules';

describe('dashboard locations page modules', () => {
  it('loads and caches the location form modal lazily', async () => {
    const first = await loadLocationFormModal();
    const second = await loadLocationFormModal();

    expect(first.LocationFormModalComponent).toBe(sentinels.locationFormModal);
    expect(second).toBe(first);
  });
});
