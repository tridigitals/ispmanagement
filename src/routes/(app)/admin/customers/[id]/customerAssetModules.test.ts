import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  assets: { name: 'customer-assets-tab' },
}));

vi.mock('./CustomerAssetsTab.svelte', () => ({
  default: sentinels.assets,
}));

import { loadCustomerAssetsTab } from './customerAssetModules';

describe('customer asset modules', () => {
  it('loads and caches the customer assets tab component on demand', async () => {
    const first = await loadCustomerAssetsTab();
    const second = await loadCustomerAssetsTab();

    expect(first).toEqual({ default: sentinels.assets });
    expect(second).toBe(first);
  });
});
