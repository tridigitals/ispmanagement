import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  networkAssetFormModal: { name: 'network-asset-form-modal' },
}));

vi.mock('./NetworkAssetFormModal.svelte', () => ({
  default: sentinels.networkAssetFormModal,
}));

import { loadNetworkAssetFormModal } from './networkAssetsPageModules';

describe('network assets page modules', () => {
  it('loads and caches the asset form modal lazily', async () => {
    const first = await loadNetworkAssetFormModal();
    const second = await loadNetworkAssetFormModal();

    expect(first.NetworkAssetFormModalComponent).toBe(sentinels.networkAssetFormModal);
    expect(second).toBe(first);
  });
});
