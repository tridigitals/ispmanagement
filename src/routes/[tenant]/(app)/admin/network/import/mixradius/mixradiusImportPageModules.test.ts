import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  wizard: { name: 'mixradius-import-wizard' },
}));

vi.mock('$lib/components/network/mixradius/MixRadiusImportWizard.svelte', () => ({
  default: sentinels.wizard,
}));

import { loadMixradiusImportWizard } from './mixradiusImportPageModules';

describe('mixradius import page modules', () => {
  it('loads and caches the mixradius import wizard lazily', async () => {
    const first = await loadMixradiusImportWizard();
    const second = await loadMixradiusImportWizard();

    expect(first.WizardComponent).toBe(sentinels.wizard);
    expect(second).toBe(first);
  });
});
