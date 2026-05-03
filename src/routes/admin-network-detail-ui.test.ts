import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('admin network detail UI cleanup', () => {
  it('keeps network detail and import surfaces on clean dark tokens', () => {
    const files = [
      'src/routes/(app)/admin/network/installations/InstallationDetailDialogs.svelte',
      'src/routes/(app)/admin/network/routers/[id]/RouterDetailDialogs.svelte',
      'src/routes/(app)/admin/network/routers/[id]/+page.svelte',
      'src/routes/(app)/admin/network/pppoe/import/+page.svelte',
      'src/routes/(app)/admin/network/import/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toContain('var(--bg-card)');
      expect(source, file).not.toContain('linear-gradient');
      expect(source, file).not.toContain('radial-gradient');
      expect(source, file).not.toContain('backdrop-filter');
      expect(source, file).not.toContain('border-radius: 18px');
      expect(source, file).toContain('var(--bg-surface)');
    }
  });

  it('keeps installation detail dialog mobile-first for dense grids', () => {
    const source = readSource(
      'src/routes/(app)/admin/network/installations/InstallationDetailDialogs.svelte',
    );

    expect(source).toMatch(/@media \(max-width: 800px\)[\s\S]*\.meta-grid[\s\S]*grid-template-columns: 1fr/);
    expect(source).toMatch(/@media \(max-width: 800px\)[\s\S]*\.step-flow[\s\S]*grid-template-columns: 1fr/);
  });
});
