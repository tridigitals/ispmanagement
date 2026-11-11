import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { hasLucideIconModule } from '$lib/utils/iconModules';
import { getLucideIconImportPath } from '$lib/utils/iconResolver';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('sidebar icon registry', () => {
  it('uses a registered lucide icon for the FTTH assets nav item', () => {
    const source = readSource('src/lib/components/layout/Sidebar.svelte');
    const match = source.match(
      /label:\s*\$t\('sidebar\.ftth_assets'\)\s*\|\|\s*'FTTH Assets'[\s\S]*?icon:\s*'([^']+)'/,
    );

    expect(match).not.toBeNull();

    const iconName = match?.[1];
    const resolvedIconName = getLucideIconImportPath(iconName);

    expect(resolvedIconName).not.toBe('help-circle');
    expect(hasLucideIconModule(resolvedIconName)).toBe(true);
  });

  it('admin panel jump button targets /v2/admin, not the dead /admin route', () => {
    const source = readSource('src/lib/components/layout/Sidebar.svelte');
    // Route (app)/admin/** sudah dihapus sejak cutover v2. Jika masih menunjuk
    // `/admin`, URL jatuh ke route dinamis `[tenant]` → salah redirect balik ke
    // superadmin (klik tombol admin panel = gagal masuk admin).
    expect(source).toContain("const adminPanelHref = '/v2/admin'");
    expect(source).not.toContain("const adminPanelHref = '/admin'");
  });

  it('admin/user toggle navigates to /v2/admin, not dead /admin', () => {
    const source = readSource('src/lib/components/layout/Sidebar.svelte');
    // Toggle admin↔user pada sidebar tenant harus menunjuk /v2/admin
    // (bukan /admin yang sudah mati). Catatan: `${tenantPrefix}/admin/...`
    // di `adminMenuSections` adalah item menu yang sah, jadi kami tidak
    // melarang seluruh substring — kami memeriksa string navigasi onclick-nya.
    expect(source).toContain(
      "goto(isUrlAdmin ? `${tenantPrefix}/dashboard` : `${tenantPrefix}/v2/admin`)",
    );
  });
});
