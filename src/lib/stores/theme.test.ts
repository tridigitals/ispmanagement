import { get } from 'svelte/store';
import { beforeEach, describe, expect, it, vi } from 'vitest';

describe('theme store', () => {
  let attrs: Record<string, string>;
  let storage: Record<string, string>;

  beforeEach(() => {
    vi.resetModules();
    attrs = {};
    storage = {};

    vi.stubGlobal('localStorage', {
      getItem: (key: string) => storage[key] ?? null,
      setItem: (key: string, value: string) => {
        storage[key] = value;
      },
      removeItem: (key: string) => {
        delete storage[key];
      },
    });

    vi.stubGlobal('document', {
      documentElement: {
        setAttribute: (key: string, value: string) => {
          attrs[key] = value;
        },
      },
    });
  });

  it('ignores saved light preference and initializes dark theme', async () => {
    storage.theme = 'light';
    const { theme } = await import('./theme');

    theme.init();

    expect(get(theme)).toBe('dark');
    expect(storage.theme).toBe('dark');
    expect(attrs['data-theme']).toBe('dark');
  });

  it('keeps dark theme when toggled', async () => {
    const { theme } = await import('./theme');

    theme.set('light' as any);
    theme.toggle();

    expect(get(theme)).toBe('dark');
    expect(storage.theme).toBe('dark');
    expect(attrs['data-theme']).toBe('dark');
  });
});
