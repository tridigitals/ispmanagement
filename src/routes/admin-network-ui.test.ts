import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('admin network UI cleanup', () => {
  it('uses clean shared surface tokens on top-level network operation pages', () => {
    const files = [
      'src/routes/(app)/admin/network/alerts/+page.svelte',
      'src/routes/(app)/admin/network/incidents/+page.svelte',
      'src/routes/(app)/admin/network/installations/+page.svelte',
      'src/routes/(app)/admin/network/ip-pools/+page.svelte',
      'src/routes/(app)/admin/network/noc/+page.svelte',
      'src/routes/(app)/admin/network/ppp-profiles/+page.svelte',
      'src/routes/(app)/admin/network/pppoe/+page.svelte',
      'src/routes/(app)/admin/network/routers/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toContain('var(--bg-card)');
      expect(source, file).not.toContain('border-radius: 18px');
      expect(source, file).not.toContain('0 12px 30px rgba(0, 0, 0, 0.2)');
      expect(source, file).toContain('var(--bg-surface)');
      expect(source, file).toContain('var(--radius-lg)');
    }
  });

  it('keeps network metric grids readable on mobile', () => {
    const files = [
      'src/routes/(app)/admin/network/alerts/+page.svelte',
      'src/routes/(app)/admin/network/installations/+page.svelte',
      'src/routes/(app)/admin/network/noc/+page.svelte',
      'src/routes/(app)/admin/network/pppoe/+page.svelte',
      'src/routes/(app)/admin/network/routers/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).toMatch(/@media \(max-width: 640px\)[\s\S]*grid-template-columns: 1fr/);
    }
  });
});
