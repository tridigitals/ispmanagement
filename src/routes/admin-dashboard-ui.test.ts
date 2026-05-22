import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('admin dashboard executive redesign', () => {
  it('uses an executive shell instead of the old generic masthead-and-grid treatment', () => {
    const source = readSource('src/routes/(app)/admin/+page.svelte');

    expect(source).toContain('dashboard-shell');
    expect(source).toContain('executive-hero');
    expect(source).toContain('kpi-strip');
    expect(source).toContain('focus-band');
    expect(source).toContain('decision-grid');
    expect(source).toContain('trend-visual');
    expect(source).toContain('trend-ring');
    expect(source).toContain('section-kicker');
    expect(source).toContain('action-rail');
    expect(source).toContain('trend-share');
  });

  it('keeps the dashboard free of decorative gradients and blur-heavy chrome', () => {
    const source = readSource('src/routes/(app)/admin/+page.svelte');

    expect(source).not.toMatch(/(?:linear|radial)-gradient/);
    expect(source).not.toContain('backdrop-filter');
    expect(source).not.toMatch(/background:\s*#(?:fff|ffffff)\b/i);
  });
});
