import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('superadmin UI cleanup', () => {
  it('keeps superadmin pages aligned with the clean dark surface system', () => {
    const files = [
      'src/routes/superadmin/+page.svelte',
      'src/routes/superadmin/audit-logs/+page.svelte',
      'src/routes/superadmin/users/+page.svelte',
      'src/routes/superadmin/plans/+page.svelte',
      'src/routes/superadmin/plans/[id]/+page.svelte',
      'src/routes/superadmin/backups/+page.svelte',
      'src/routes/superadmin/tenants/+page.svelte',
      'src/routes/superadmin/invoices/+page.svelte',
      'src/routes/superadmin/invoices/[id]/+page.svelte',
      'src/routes/superadmin/settings/+page.svelte',
      'src/routes/superadmin/radius/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toMatch(/(?:linear|radial)-gradient/);
      expect(source, file).not.toContain('backdrop-filter');
      expect(source, file).not.toMatch(/border-radius:\s*(?:1[6-9]|[2-9][0-9])px/);
      expect(source, file).not.toMatch(/background:\s*#ffffff/);
      expect(source, file).toContain('var(--bg-surface)');
    }
  });
});
