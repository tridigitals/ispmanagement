import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('dashboard user UI cleanup', () => {
  it('keeps shared stat cards within the clean dark design language', () => {
    const source = readSource('src/lib/components/dashboard/StatsCard.svelte');

    expect(source).not.toContain('linear-gradient');
    expect(source).not.toContain('border-radius: 18px');
    expect(source).not.toContain('translateY(-2px)');
    expect(source).toContain('var(--bg-surface)');
  });

  it('keeps dashboard user surfaces free of decorative gradients', () => {
    const files = [
      'src/routes/(app)/dashboard/+page.svelte',
      'src/routes/(app)/dashboard/services/+page.svelte',
      'src/routes/(app)/notifications/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);
      expect(source, file).not.toContain('background: linear-gradient');
      expect(source, file).not.toContain('border-radius: 16px');
    }
  });
});
