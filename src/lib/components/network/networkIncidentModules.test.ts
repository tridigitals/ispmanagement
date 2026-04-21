import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  detailDrawer: { name: 'incident-detail-drawer' },
  simulateDrawer: { name: 'incident-simulate-drawer' },
}));

vi.mock('$lib/components/network/IncidentDetailDrawer.svelte', () => ({
  default: sentinels.detailDrawer,
}));

vi.mock('$lib/components/network/IncidentSimulateDrawer.svelte', () => ({
  default: sentinels.simulateDrawer,
}));

import {
  loadIncidentDetailDrawer,
  loadIncidentSimulateDrawer,
} from './networkIncidentModules';

describe('network incident modules', () => {
  it('loads and caches the incident detail drawer on demand', async () => {
    const first = await loadIncidentDetailDrawer();
    const second = await loadIncidentDetailDrawer();

    expect(first).toEqual({
      IncidentDetailDrawerComponent: sentinels.detailDrawer,
    });
    expect(second).toBe(first);
  });

  it('loads and caches the incident simulate drawer on demand', async () => {
    const first = await loadIncidentSimulateDrawer();
    const second = await loadIncidentSimulateDrawer();

    expect(first).toEqual({
      IncidentSimulateDrawerComponent: sentinels.simulateDrawer,
    });
    expect(second).toBe(first);
  });
});
