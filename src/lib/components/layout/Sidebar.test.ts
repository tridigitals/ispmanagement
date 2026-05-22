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
});
