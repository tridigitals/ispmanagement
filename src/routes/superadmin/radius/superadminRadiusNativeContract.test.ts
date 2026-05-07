import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('superadmin radius native contract', () => {
  it('removes legacy db field usage from the managed radius page', () => {
    const page = readSource('src/routes/superadmin/radius/+page.svelte');

    expect(page).not.toContain('db_host');
    expect(page).not.toContain('db_name');
    expect(page).not.toContain('db_user');
    expect(page).not.toContain('db_password');
  });

  it('uses endpoint and ports terminology in the managed radius page', () => {
    const page = readSource('src/routes/superadmin/radius/+page.svelte');

    expect(page).toContain('runtimeStatus.advertised_host');
    expect(page).toContain('item.radius_host');
    expect(page).toContain('item.auth_port');
    expect(page).toContain('item.acct_port');
    expect(page).not.toContain("superadmin.radius.columns.database");
  });

  it('removes the legacy server management surface from the superadmin radius page', () => {
    const page = readSource('src/routes/superadmin/radius/+page.svelte');
    const modules = readSource('src/routes/superadmin/radius/superadminRadiusPageModules.ts');

    expect(page).not.toContain("superadmin.radius.actions.new_server");
    expect(page).not.toContain("superadmin.radius.sections.servers");
    expect(modules).not.toContain('ServerFormModal');
  });
});
