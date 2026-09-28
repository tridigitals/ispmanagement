import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

const PAGE = 'src/routes/(v2)/v2/admin/team/+page.svelte';
const API = 'src/lib/api/team.ts';

/**
 * Dua permintaan pemilik yang dikunci di sini:
 *
 * 1. Form tim harus punya STATUS akun, supaya verifikasi email bisa
 *    dilakukan manual. Sebelumnya tidak ada satu pun cara di UI untuk
 *    mengubah `email_verified_at`, padahal `add_member` tidak pernah
 *    mengisinya — akun buatan admin langsung terkunci begitu tenant
 *    menyalakan wajib-verifikasi email (auth_service: "email_unverified").
 *
 * 2. Daftar tim harus punya aksi EDIT, bukan hanya ubah role + hapus.
 */
describe('tim v2 — status akun di form & aksi Edit di daftar', () => {
  it('form Tambah mengirim status akun & status verifikasi email', () => {
    const src = readSource(PAGE);
    const s = src.indexOf('async function kirimUndangan()');
    const body = src.slice(s, src.indexOf('\n  }', s));

    expect(body, 'harus memakai api.team.add').toContain('api.team.add(');
    expect(body, 'harus mengirim isActive').toContain('isActive:');
    expect(body, 'harus mengirim emailVerified').toContain('emailVerified:');
  });

  it('form Edit mengirim status akun & verifikasi, bukan hanya role', () => {
    const src = readSource(PAGE);
    const s = src.indexOf('async function simpanEdit()');
    expect(s, 'simpanEdit harus ada').toBeGreaterThan(-1);
    const body = src.slice(s, src.indexOf('\n  }', s));

    expect(body).toContain('api.team.update(');
    expect(body).toContain('isActive:');
    expect(body).toContain('emailVerified:');
  });

  it('role TIDAK dipaksa ikut dikirim saat status saja yang diubah', () => {
    const src = readSource(PAGE);
    const body = src.slice(src.indexOf('async function simpanEdit()'));

    // Kirim role hanya bila user benar-benar mengubahnya — kalau tidak,
    // user yang tidak berhak atas role itu akan kena 403 saat sekadar
    // menonaktifkan akun.
    expect(body, 'harus memakai flag sentuh').toContain('editRoleTouched');
    expect(body).toMatch(/roleId:\s*gantiRole\s*\?\s*editRoleId\s*:\s*undefined/);
  });

  it('aksi baris menyediakan Edit (bukan hanya change role + delete)', () => {
    const src = readSource(PAGE);
    expect(src, 'label Edit harus dipakai').toContain("$t('admin.team.v2.edit')");
    expect(src, 'handler bukaEdit harus dipanggil aksi primary').toContain('onclick: () => bukaEdit(m)');
  });

  it('status akun terlihat langsung di daftar tabel', () => {
    const src = readSource(PAGE);
    expect(src, 'kolom status harus terdaftar').toContain("key: 'status'");
    expect(src, 'sel status harus dirender').toContain("c.key === 'status'");
    expect(src, 'badge belum terverifikasi').toContain("$t('admin.team.v2.st_unverified')");
  });

  it('tombol Simpan menolak saat tidak ada perubahan', () => {
    const src = readSource(PAGE);
    expect(src).toContain('adaPerubahan');
    expect(src, 'tombol Simpan memakai adaPerubahan').toMatch(/disabled=\{!adaPerubahan\}/);
  });

  it('API client MENGIRIM status lewat body (safeInvoke menyaring token/id)', () => {
    const src = readSource(API);
    expect(src).toContain('is_active: opts?.isActive');
    expect(src).toContain('email_verified: opts?.emailVerified');
    expect(src).toContain('is_active: data.isActive');
    expect(src).toContain('email_verified: data.emailVerified');
  });
});
