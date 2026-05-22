import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('customer subscriptions tab cleanup', () => {
  it('removes the lifecycle summary block and keeps the lighter overview strip', () => {
    const source = readSource('src/routes/(app)/admin/customers/[id]/CustomerSubscriptionsTab.svelte');

    expect(source).toContain('summary-strip');
    expect(source).not.toContain('Ringkasan lifecycle layanan');
    expect(source).not.toContain('lifecycle-observability');
    expect(source).not.toContain('observability-grid');
    expect(source).not.toContain('aging-row');
  });
});
