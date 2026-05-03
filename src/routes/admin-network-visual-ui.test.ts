import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('admin network visual UI cleanup', () => {
  it('keeps map, wallboard, and MixRadius loader free of decorative gradients and hardcoded white panels', () => {
    const files = [
      'src/routes/(app)/admin/network/map/+page.svelte',
      'src/routes/(app)/admin/network/noc/wallboard/+page.svelte',
      'src/routes/(app)/admin/network/import/mixradius/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toContain('linear-gradient');
      expect(source, file).not.toContain('backdrop-filter');
      expect(source, file).not.toContain('border-radius: 18px');
      expect(source, file).not.toMatch(/background:\s*#ffffff/);
      expect(source, file).toContain('var(--bg-surface)');
    }
  });
});
