import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('app detail UI cleanup', () => {
  it('keeps profile, support, subscription, and detail pages visually restrained', () => {
    const files = [
      'src/lib/components/profile/ProfileSurface.svelte',
      'src/lib/components/profile/ProfileModal.svelte',
      'src/routes/(app)/support/[id]/+page.svelte',
      'src/routes/(app)/admin/support/[id]/+page.svelte',
      'src/routes/(app)/admin/subscription/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toMatch(/(?:linear|radial)-gradient/);
      expect(source, file).not.toContain('backdrop-filter');
      expect(source, file).not.toMatch(/border-radius:\s*(?:1[6-9]|[2-9][0-9])px/);
      expect(source, file).not.toMatch(/background:\s*#(?:fff|ffffff)\b/i);
      expect(source, file).toContain('var(--bg-surface)');
    }
  });
});
