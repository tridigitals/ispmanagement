import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('admin operational UI cleanup', () => {
  it('keeps high-traffic admin surfaces away from glass and decorative gradients', () => {
    const files = [
      'src/routes/(app)/admin/settings/+page.svelte',
      'src/routes/(app)/admin/team/+page.svelte',
      'src/routes/(app)/admin/support/+page.svelte',
      'src/routes/(app)/admin/services/+page.svelte',
      'src/routes/(app)/admin/backups/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toContain('--glass');
      expect(source, file).not.toContain('linear-gradient');
      expect(source, file).not.toContain('radial-gradient');
      expect(source, file).not.toContain('backdrop-filter');
    }
  });

  it('keeps admin metric grids readable on narrow mobile screens', () => {
    const files = [
      'src/routes/(app)/admin/team/+page.svelte',
      'src/routes/(app)/admin/support/+page.svelte',
      'src/routes/(app)/admin/invoices/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).toMatch(/@media \(max-width: 640px\)[\s\S]*grid-template-columns: 1fr/);
    }
  });
});
