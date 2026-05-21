import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('topbar global search wiring', () => {
  it('wires the shared topbar search input to the global search store and result panel', () => {
    const topbar = readSource('src/lib/components/layout/Topbar.svelte');
    const panel = readSource('src/lib/components/layout/TopbarGlobalSearchPanel.svelte');

    expect(topbar).toContain("from '$lib/stores/globalSearch'");
    expect(topbar).toContain("from '$lib/search/globalSearchService'");
    expect(topbar).toContain('<TopbarGlobalSearchPanel');
    expect(topbar).toContain('class="center-section"');
    expect(topbar).toContain('search-shortcut');
    expect(topbar).toContain('$globalSearch.open && $globalSearch.query.trim().length > 0');
    expect(topbar).toContain('latestSearchRequestId');
    expect(topbar).toContain('if (requestId !== latestSearchRequestId)');
    expect(panel).toContain('search-results-panel');
    expect(panel).toContain('search-group-label');
  });
});
