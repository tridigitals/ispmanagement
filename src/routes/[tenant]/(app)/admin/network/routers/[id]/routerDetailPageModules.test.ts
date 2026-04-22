import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  routerDetailDialogs: { name: 'router-detail-dialogs' },
}));

vi.mock('./RouterDetailDialogs.svelte', () => ({
  default: sentinels.routerDetailDialogs,
}));

import { loadRouterDetailDialogs } from './routerDetailPageModules';

describe('router detail page modules', () => {
  it('loads and caches router detail dialogs lazily', async () => {
    const first = await loadRouterDetailDialogs();
    const second = await loadRouterDetailDialogs();

    expect(first.RouterDetailDialogsComponent).toBe(sentinels.routerDetailDialogs);
    expect(second).toBe(first);
  });
});
