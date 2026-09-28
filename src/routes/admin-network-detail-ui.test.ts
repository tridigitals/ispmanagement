import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

/*
  Catatan pensiun halaman lama: file-file (app) yang dulu dijaga di sini
  (InstallationDetailDialogs.svelte, dan halaman import/routers versi (app))
  sudah dihapus. Penjaga dialihkan ke padanan v2 / komponen bersama yang benar
  dirender, supaya tidak ada test yang mengunci kode mati.
*/
describe('admin network detail UI cleanup', () => {
  it('keeps network detail and import surfaces on restrained surfaces', () => {
    const files = [
      'src/lib/components/network/RouterDetailDialogs.svelte',
      'src/routes/(v2)/v2/admin/network/routers/[id]/+page.svelte',
      'src/routes/(v2)/v2/admin/network/pppoe/import/+page.svelte',
      'src/routes/(v2)/v2/admin/network/import/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toContain('var(--bg-card)');
      expect(source, file).not.toContain('linear-gradient');
      expect(source, file).not.toContain('radial-gradient');
      expect(source, file).not.toContain('backdrop-filter');
      expect(source, file).not.toContain('border-radius: 18px');
    }
  });

  it('keeps the shared router detail dialog usable and restrained', () => {
    const source = readSource('src/lib/components/network/RouterDetailDialogs.svelte');

    expect(source).toContain('var(--bg-surface)');
    expect(source).toMatch(/@media \(max-width: \d+px\)/);
  });
});
