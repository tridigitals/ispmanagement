import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  Lightbox: { name: 'lightbox-component' },
}));

vi.mock('$lib/components/ui/Lightbox.svelte', () => ({
  default: sentinels.Lightbox,
}));

import { loadLightboxModule } from './lightboxModule';

describe('lightbox module loader', () => {
  it('loads and caches the lightbox component on demand', async () => {
    const first = await loadLightboxModule();
    const second = await loadLightboxModule();

    expect(first).toEqual({
      LightboxComponent: sentinels.Lightbox,
    });
    expect(second).toBe(first);
  });
});
