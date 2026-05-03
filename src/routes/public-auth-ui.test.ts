import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const publicAuthPages = [
  'src/routes/forgot-password/+page.svelte',
  'src/routes/unauthorized/+page.svelte',
  'src/routes/verify-email/+page.svelte',
];

function readPage(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('public auth UI', () => {
  it('does not use decorative emoji or text glyphs for status icons', () => {
    for (const path of publicAuthPages) {
      const source = readPage(path);

      expect(source, path).not.toMatch(/[🔑📩🛡️✓✕]/u);
    }
  });
});
