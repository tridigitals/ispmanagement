import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('admin network detail UI cleanup', () => {
  it('keeps network detail and import surfaces on clean dark tokens', () => {
    const files = [
      'src/lib/components/network/RouterDetailDialogs.svelte',
      'src/routes/(app)/admin/network/routers/[id]/+page.svelte',
      'src/routes/(app)/admin/network/pppoe/import/+page.svelte',
      'src/routes/(app)/admin/network/import/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toContain('var(--bg-card)');
      expect(source, file).not.toContain('linear-gradient');
      expect(source, file).not.toContain('radial-gradient');
      expect(source, file).not.toContain('backdrop-filter');
      expect(source, file).not.toContain('border-radius: 18px');
      expect(source, file).toContain('var(--bg-surface)');
    }
  });

  /* InstallationDetailDialogs.svelte sudah dihapus: tidak pernah dirender siapa
     pun (hanya dimuat oleh modul lazy yang juga tidak dipakai), sementara
     InstallationDetailModal.svelte adalah dialog yang benar-benar tampil.
     Penjagaan gaya dialihkan ke file yang hidup supaya tidak mengunci kode mati. */
  it('keeps the rendered installation detail dialog mobile-first for dense grids', () => {
    const source = readSource(
      'src/routes/(app)/admin/network/installations/InstallationDetailModal.svelte',
    );

    /* Breakpoint nyatanya 640px (bukan 800px seperti file mati yang dulu diuji).
       Tidak dipatok angka persisnya supaya penyesuaian breakpoint yang sah tidak
       memicu kegagalan palsu — yang penting grid padat benar-benar runtuh. */
    expect(source).toMatch(/@media \(max-width: \d+px\)[^{]*\{[^}]*\.meta-grid[^}]*grid-template-columns: 1fr/);
  });
});
