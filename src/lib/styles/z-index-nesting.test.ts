import { describe, expect, it } from 'vitest';
import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

/**
 * Invarian lapisan z-index.
 *
 * Latar: dua bug nyata yang dilaporkan.
 *
 *  1. Map picker pada halaman OLT (v2) memakai `z-[70]`, sedangkan `Modal`
 *     memakai z-index 100. Picker itu dibuka DARI DALAM modal form, jadi ia
 *     berada DI BAWAH modal induknya sendiri dan tidak bisa dipakai.
 *  2. `ConfirmDialog` dulu hanya membungkus `Modal`, sehingga mewarisi z-index
 *     100 yang sama. Di dalam modal ber-z-index lebih tinggi (NotificationModal
 *     dulu 1200), dialog konfirmasi tertutup di belakang induknya — tombol
 *     Konfirmasi/Batal tak bisa diklik. Terjadi pada "baca semua" dan
 *     "hapus semua" notifikasi.
 *
 * Akarnya sama: overlay yang SALING BERSARANG tidak punya urutan yang pasti,
 * sehingga hasilnya bergantung urutan render DOM. Test ini mengunci urutannya
 * secara eksplisit supaya tidak bisa rusak diam-diam lagi.
 */

const SRC = resolve(process.cwd(), 'src');
const read = (p: string) => readFileSync(resolve(SRC, p), 'utf8');

/** Ambil nilai token dari blok :root di global.css. */
function token(name: string): number {
  const css = read('lib/styles/global.css');
  const m = css.match(new RegExp(`--${name}:\\s*(\\d+)`));
  expect(m, `token --${name} harus ada di global.css`).not.toBeNull();
  return Number(m![1]);
}

/** z-index pertama yang dipakai sebuah selector, dalam bentuk angka atau token. */
function zIndexOf(file: string, needle: string): number {
  const src = read(file);
  const i = src.indexOf(needle);
  expect(i, `"${needle}" harus ada di ${file}`).toBeGreaterThan(-1);
  const window = src.slice(i, i + 400);
  const varMatch = window.match(/z-index:\s*var\(--([a-z-]+)/);
  if (varMatch) return token(varMatch[1]);
  const numMatch = window.match(/z-index:\s*(\d+)/);
  expect(numMatch, `z-index tidak ditemukan setelah "${needle}" di ${file}`).not.toBeNull();
  return Number(numMatch![1]);
}

describe('skala z-index', () => {
  it('token terurut menaik sesuai maksudnya', () => {
    expect(token('z-base')).toBeLessThan(token('z-drawer-backdrop'));
    expect(token('z-drawer-backdrop')).toBeLessThan(token('z-drawer'));
    expect(token('z-drawer')).toBeLessThan(token('z-dialog-in-drawer'));
    expect(token('z-dialog-in-drawer')).toBeLessThan(token('z-modal'));
    expect(token('z-modal')).toBeLessThan(token('z-modal-nested'));
    expect(token('z-modal-nested')).toBeLessThan(token('z-confirm'));
    expect(token('z-confirm')).toBeLessThan(token('z-confirm-nested'));
    expect(token('z-confirm-nested')).toBeLessThan(token('z-app-chrome'));
    expect(token('z-app-chrome')).toBeLessThan(token('z-app-overlay'));
    expect(token('z-app-overlay')).toBeLessThan(token('z-critical-overlay'));
  });
});

describe('ConfirmDialog selalu di atas modal', () => {
  it('dipakai lewat token --z-confirm, bukan mewarisi Modal', () => {
    const src = read('lib/components/ui/ConfirmDialog.svelte');
    // Dulu: `<Modal {show} ...>` -> mewarisi z-index Modal (100).
    expect(src, 'ConfirmDialog tidak boleh membungkus Modal').not.toMatch(/<Modal\b/);
    expect(src).toContain('z-index: var(--z-confirm');
  });

  it('lebih tinggi dari --z-modal', () => {
    expect(zIndexOf('lib/components/ui/ConfirmDialog.svelte', '.confirm-backdrop')).toBeGreaterThan(
      token('z-modal'),
    );
  });

  /**
   * Setiap modal yang bisa membuka ConfirmDialog harus ber-z-index di BAWAH
   * --z-confirm. Kalau ada modal yang naik di atasnya, dialog konfirmasinya
   * kembali tidak terlihat.
   */
  it('semua modal pemakai ConfirmDialog berada di bawah --z-confirm', () => {
    const kandidat = [
      'lib/components/notifications/NotificationModal.svelte',
      'lib/components/profile/ProfileModal.svelte',
      'lib/components/ui/FileManager.svelte',
      'routes/(app)/admin/network/olts/[id]/+page.svelte',
      'routes/(app)/admin/invoices/[id]/+page.svelte',
      'routes/(app)/admin/roles/+page.svelte',
      'routes/superadmin/settings/+page.svelte',
    ];
    const confirm = token('z-confirm');
    const pelanggar: string[] = [];

    for (const f of kandidat) {
      if (!existsSync(resolve(SRC, f))) continue;
      const src = read(f);
      if (!src.includes('ConfirmDialog')) continue;
      // Ambil SEMUA z-index eksplisit di file; tak satu pun boleh >= --z-confirm.
      for (const m of src.matchAll(/z-index:\s*var\(--([a-z-]+)/g)) {
        const v = token(m[1]);
        if (v >= confirm) pelanggar.push(`${f}: --${m[1]} (${v}) >= --z-confirm (${confirm})`);
      }
      for (const m of src.matchAll(/z-index:\s*(\d{3,})/g)) {
        if (Number(m[1]) >= confirm) pelanggar.push(`${f}: z-index ${m[1]} >= ${confirm}`);
      }
    }
    expect(pelanggar, 'modal pemakai ConfirmDialog naik di atas dialog konfirmasi').toEqual([]);
  });
});

describe('map picker di atas modal induknya', () => {
  it('(v2) OLT: picker tidak lagi di bawah --z-modal', () => {
    const f = 'routes/(v2)/v2/admin/network/olts/+page.svelte';
    const src = read(f);
    // Hanya periksa KODE, bukan komentar: komentar penjelas menyebut nilai lama
    // (`z-[70]`) sebagai dokumentasi bug, jadi jangan ikut diuji. File ini tidak
    // punya blok <style> (styling via Tailwind), jadi buang komentar HTML saja.
    const kode = src
      .replace(/<!--[\s\S]*?-->/g, '')
      .split('<style')[0];
    // Bug: `z-[70]` (< 100) padahal dibuka dari dalam Modal (100).
    expect(kode, 'z-[70] pada picker akan tertutup modal').not.toContain('z-[70]');
    expect(kode).toContain('var(--z-modal-nested');

    // Backdrop picker: pastikan nilainya > --z-modal. Anchor pada class-nya —
    // memakai `showMapPicker = false` malah menangkap definisi fungsi di
    // <script> (yang muncul lebih dulu), bukan markup.
    const i = kode.indexOf('place-items-center bg-black/40');
    expect(i, 'markup backdrop picker harus ada').toBeGreaterThan(-1);
    const sekitar = kode.slice(i, i + 300);
    const varMatch = sekitar.match(/z-index:\s*var\(--([a-z-]+)/);
    const numMatch = sekitar.match(/z-index:\s*(\d+)/);
    const nilai = varMatch ? token(varMatch[1]) : Number(numMatch?.[1]);
    expect(Number.isFinite(nilai), 'z-index backdrop picker harus terbaca').toBe(true);
    expect(nilai).toBeGreaterThan(token('z-modal'));
  });

  it('(app) OLT: backdrop picker eksplisit di atas modal', () => {
    const f = 'routes/(app)/admin/network/olts/+page.svelte';
    const src = read(f);
    expect(src).toContain('map-picker-backdrop');
    expect(src).toMatch(/\.modal-backdrop\.map-picker-backdrop\s*\{[^}]*z-index:\s*var\(--z-modal-nested/);
  });
});
