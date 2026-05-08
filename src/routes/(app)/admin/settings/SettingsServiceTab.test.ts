import { describe, expect, it } from 'vitest';

describe('settings service tab module', () => {
  it('can be imported lazily', async () => {
    const module = await import('./SettingsServiceTab.svelte');

    expect(module.default).toBeTruthy();
  });
});
