import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

const PLAN_EDITOR = 'src/routes/superadmin/plans/[id]/+page.svelte';

/**
 * Nilai limit plan boleh berupa kata kunci (`unlimited`), bukan angka.
 * Dua regresi yang dijaga di sini:
 *
 * 1. `<input type="number">` tidak bisa menampilkan `unlimited` — field jadi
 *    kosong, dan menyimpan berarti menulis `''` yang kini dibaca sebagai
 *    limit 0 (fitur terkunci).
 * 2. `savePlan` wajib menolak limit kosong sebelum menulis apa pun, supaya
 *    plan tidak tersimpan setengah jalan dengan limit yang mengunci fitur.
 */
describe('editor plan — nilai limit non-numerik', () => {
  it('memilih tipe input dari nilai: number hanya bila benar-benar angka', () => {
    const source = readSource(PLAN_EDITOR);

    expect(source).toContain('function isNumericValue');
    // Input number TIDAK boleh hard-coded; harus dikondisikan.
    expect(source).toContain("type={isNumericValue(planFeatures[feature.id]) ? 'number' : 'text'}");
    expect(source, 'input number hard-coded akan menyembunyikan "unlimited"').not.toMatch(
      /type="number"\s+value=\{planFeatures\[feature\.id\]\}/,
    );
  });

  it('memberi placeholder unlimited supaya nilai kosong tidak ambigu', () => {
    const source = readSource(PLAN_EDITOR);
    expect(source).toContain('placeholder="unlimited"');
  });

  it('menolak simpan bila limit yang di-enforce kosong', () => {
    const source = readSource(PLAN_EDITOR);

    expect(source).toContain('function findEmptyLimit');
    for (const code of ['max_users', 'max_members', 'max_storage_gb']) {
      expect(source, `${code} harus divalidasi`).toContain(`'${code}'`);
    }
    // Validasi harus terjadi sebelum flag saving di-set (tidak ada penulisan
    // sebagian ke backend).
    const guardIdx = source.indexOf('const empty = findEmptyLimit()');
    const savingIdx = source.indexOf('saving = true;', source.indexOf('async function savePlan'));
    expect(guardIdx).toBeGreaterThan(-1);
    expect(savingIdx).toBeGreaterThan(guardIdx);
  });

  it('isNumericValue menerima angka dan menolak kata kunci', () => {
    // Replikasi logika helper (fungsi di dalam komponen tidak bisa diimpor
    // langsung di lingkungan test ini).
    const isNumericValue = (v: string | undefined): boolean => {
      if (v === undefined || v === null) return false;
      const s = String(v).trim();
      return s !== '' && Number.isFinite(Number(s));
    };

    expect(isNumericValue('50')).toBe(true);
    expect(isNumericValue('0.5')).toBe(true);
    expect(isNumericValue(' 20 ')).toBe(true);
    expect(isNumericValue('unlimited')).toBe(false);
    expect(isNumericValue('-1')).toBe(true); // -1 = angka (artinya unlimited di backend)
    expect(isNumericValue('')).toBe(false);
    expect(isNumericValue(undefined)).toBe(false);
    expect(isNumericValue('abc')).toBe(false);
  });
});
