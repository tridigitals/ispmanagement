import { describe, expect, it } from 'vitest';

describe('settings service tab module', () => {
  // The dynamic svelte import below triggers a full svelte SSR compile during
  // the test. Under parallel test pool execution (170+ files), Vite's
  // transform queue is heavily contended and a single .svelte compile can
  // exceed the default 5s timeout, even though it runs in <1s in isolation.
  // The bump only applies to this test and reflects environment reality, not
  // a logic bug. See the analogous SuperAdmin settings test for the same
  // pattern.
  it(
    'can be imported lazily',
    async () => {
      const module = await import('./SettingsServiceTab.svelte');

      expect(module.default).toBeTruthy();
    },
    20000,
  );
});
