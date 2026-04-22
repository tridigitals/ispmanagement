import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  routerFormModal: { name: 'router-form-modal' },
}));

vi.mock('./RouterFormModal.svelte', () => ({
  default: sentinels.routerFormModal,
}));

import { loadRouterFormModal } from './routersPageModules';

describe('routers page modules', () => {
  it('loads and caches the router form modal lazily', async () => {
    const first = await loadRouterFormModal();
    const second = await loadRouterFormModal();

    expect(first.RouterFormModalComponent).toBe(sentinels.routerFormModal);
    expect(second).toBe(first);
  });
});
