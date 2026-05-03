import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

function collectSvelteFiles(dir: string): string[] {
  const entries = readdirSync(dir);
  const files: string[] = [];

  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);

    if (stat.isDirectory()) {
      files.push(...collectSvelteFiles(fullPath));
      continue;
    }

    if (entry.endsWith('.svelte') && !entry.endsWith('.svelte.disabled')) {
      files.push(fullPath);
    }
  }

  return files;
}

describe('route radius token cleanup', () => {
  it('uses radius tokens instead of large hardcoded card radii in runtime routes', () => {
    const routesDir = resolve(process.cwd(), 'src/routes');
    const files = collectSvelteFiles(routesDir);
    const largeCardRadius = /border-radius:\s*(?:1[6-9]|2[0-9]|3[0-9])px/;

    for (const file of files) {
      const source = readFileSync(file, 'utf8');
      expect(source, file).not.toMatch(largeCardRadius);
    }
  });
});
