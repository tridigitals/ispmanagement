import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

const V2_PAGE = 'src/routes/(v2)/v2/admin/network/installations/+page.svelte';

/** Ambil isi satu fungsi async dari source (dari `async function nama(` sampai `\n  }`). */
function functionBody(source: string, signature: string): string {
  const start = source.indexOf(signature);
  expect(start, `${signature} tidak ditemukan`).toBeGreaterThan(-1);
  const rest = source.slice(start);
  const end = rest.indexOf('\n  }\n');
  return end === -1 ? rest : rest.slice(0, end);
}

/**
 * Bug yang dikunci di sini: user memilih teknisi di dropdown, menekan "Mulai",
 * lalu ditolak dengan "Tentukan teknisi sebelum memulai pengerjaan" — padahal
 * teknisi sudah dipilih.
 *
 * Sebabnya: `onchange` dropdown hanya menulis state lokal (`formAssignee`),
 * TIDAK memanggil API. `startWo()` lalu memanggil `start` tanpa pernah
 * menyimpan `assigned_to`, sehingga backend melihat `assigned_to` kosong di DB
 * dan menolak (`Set assignee before starting work order`). Tombol Mulai pun
 * muncul karena gate-nya juga membaca state lokal, jadi UI terlihat sah.
 *
 * Perbaikannya: simpan rencana (assign → assigned_to + scheduled_at) lebih dulu,
 * baru mulai. Pola ini sama dengan halaman (app) yang memanggil `savePlan()`
 * sebelum `setStatus(..., 'start', ...)`.
 */
describe('mulai instalasi — teknisi wajib tersimpan sebelum start', () => {
  it('startWo memanggil assign sebelum start', () => {
    const body = functionBody(readSource(V2_PAGE), 'async function startWo()');

    expect(body, 'startWo harus menyimpan teknisi').toContain('api.workOrders.assign(');
    expect(body, 'startWo harus memulai pengerjaan').toContain('api.workOrders.start(');

    const assignIdx = body.indexOf('api.workOrders.assign(');
    const startIdx = body.indexOf('api.workOrders.start(');
    expect(assignIdx, 'assign harus dipanggil LEBIH DULU dari start').toBeLessThan(startIdx);
  });

  it('assign di startWo membawa teknisi DAN jadwal', () => {
    const body = functionBody(readSource(V2_PAGE), 'async function startWo()');

    // Tanpa assigned_to, backend tetap menolak; tanpa scheduled_at, backend
    // menolak dengan "Set installation schedule before starting".
    expect(body).toMatch(/assigned_to:\s*formAssignee/);
    expect(body).toMatch(/scheduled_at:\s*new Date\(formSchedule\)\.toISOString\(\)/);
  });

  it('gate tombol Mulai & startWo memakai sumber yang sama', () => {
    const source = readSource(V2_PAGE);
    // Tombol Mulai hanya dirender bila teknisi + jadwal terisi...
    expect(source).toContain("active.status === 'pending' && formAssignee && formSchedule");
    // ...dan startWo memakai guard yang sama, supaya UI tidak pernah menampilkan
    // tombol yang pasti gagal.
    const body = functionBody(source, 'async function startWo()');
    expect(body).toContain('if (!formAssignee || !formSchedule)');
  });

  it('dropdown teknisi tetap hanya menulis state (assign dipicu saat simpan/mulai)', () => {
    const source = readSource(V2_PAGE);
    // Memanggil API langsung di onchange akan menyimpan setengah jalan
    // (teknisi tersimpan, jadwal belum) dan memicu reload yang menutup form.
    expect(source).toContain("onchange={(v) => (formAssignee = String(v ?? ''))}");
    expect(source, 'onchange tidak boleh memanggil assign').not.toMatch(
      /onchange=\{\(v\)\s*=>\s*[^}]*workOrders\.assign/,
    );
  });
});
