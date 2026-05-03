import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const runtimeRoots = ['src/routes', 'src/lib/components'];
const emojiPattern = /[\u{1f300}-\u{1faff}\u{2600}-\u{27bf}]/u;

function collectSvelteFiles(dir: string): string[] {
  const entries = readdirSync(dir);
  const files: string[] = [];

  for (const entry of entries) {
    const path = join(dir, entry);
    const stat = statSync(path);

    if (stat.isDirectory()) {
      files.push(...collectSvelteFiles(path));
      continue;
    }

    if (entry.endsWith('.svelte')) {
      files.push(path);
    }
  }

  return files;
}

describe('runtime UI emoji cleanup', () => {
  it('keeps runtime Svelte surfaces free of decorative emoji symbols', () => {
    const files = runtimeRoots.flatMap(collectSvelteFiles);

    for (const file of files) {
      const source = readFileSync(file, 'utf8');
      expect(source, relative(process.cwd(), file)).not.toMatch(emojiPattern);
    }
  });
});
