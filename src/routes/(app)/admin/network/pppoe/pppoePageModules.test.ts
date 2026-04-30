import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  pppoeAccountModal: { name: 'pppoe-account-modal' },
}));

vi.mock('./PppoeAccountModal.svelte', () => ({
  default: sentinels.pppoeAccountModal,
}));

import { loadPppoeAccountModal } from './pppoePageModules';

describe('pppoe page modules', () => {
  it('loads and caches the pppoe account modal lazily', async () => {
    const first = await loadPppoeAccountModal();
    const second = await loadPppoeAccountModal();

    expect(first.PppoeAccountModalComponent).toBe(sentinels.pppoeAccountModal);
    expect(second).toBe(first);
  });
});
