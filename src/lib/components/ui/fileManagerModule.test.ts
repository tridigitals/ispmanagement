import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  FileManager: { name: 'file-manager-component' },
}));

vi.mock('$lib/components/ui/FileManager.svelte', () => ({
  default: sentinels.FileManager,
}));

import { loadFileManagerModule } from './fileManagerModule';

describe('file manager module loader', () => {
  it('loads and caches the file manager component on demand', async () => {
    const first = await loadFileManagerModule();
    const second = await loadFileManagerModule();

    expect(first).toEqual({
      FileManagerComponent: sentinels.FileManager,
    });
    expect(second).toBe(first);
  });
});
