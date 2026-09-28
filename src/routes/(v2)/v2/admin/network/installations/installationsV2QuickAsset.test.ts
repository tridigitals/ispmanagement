import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

/**
 * Regresi: halaman Instalasi v2 HARUS punya cara membuat aset FTTH.
 *
 * Latar — dua masalah nyata yang ditemukan:
 *
 *  1. Rute lama `(app)/admin/network/installations` di-redirect ke v2
 *     (legacyV2Redirect.ts). Implementasi "buat aset dari instalasi" yang
 *     lengkap hanya ada di halaman lama, jadi fitur itu tidak terjangkau
 *     pengguna: v2 hanya menyediakan dropdown memilih aset yang SUDAH ada.
 *
 *  2. Backend sudah mengirim `location_latitude`/`location_longitude`
 *     (work_orders.rs), tetapi tipe FE tidak memuatnya sehingga nilainya
 *     dibuang dan koordinat aset harus diketik manual.
 *
 * Keduanya mudah terulang lagi tanpa test yang mengunci.
 */

const SRC = resolve(process.cwd(), 'src');
const read = (p: string) => readFileSync(resolve(SRC, p), 'utf8');

const V2_ACCOUNTS_PAGE = 'routes/(v2)/v2/admin/network/installations/+page.svelte';

describe('halaman instalasi v2 bisa membuat aset FTTH', () => {
  const src = read(V2_ACCOUNTS_PAGE);

  it('memakai modul quick-asset bersama, bukan menyalin logika', () => {
    expect(src).toContain("from '$lib/utils/installationQuickAsset'");
    expect(src).toContain('buildInstallationQuickAssetDraftFromContext');
    expect(src).toContain('buildInstallationQuickAssetPayload');
  });

  it('memakai SATU komponen form bersama, bukan menyalin markup', () => {
    // Markup form ini dulu terduplikasi di dua halaman dan isinya berbeda —
    // halaman (app) mengisi nama, v2 sempat tidak. Sekarang satu komponen.
    expect(src).toContain('InstallationQuickAssetPanel');
    expect(src).toContain("from '$lib/components/installations/InstallationQuickAssetPanel.svelte'");
    // Kalau blok markup lama kembali, ini gagal.
    expect(src).not.toContain('id="qa-lat"');
    expect(src).not.toContain('id="qa-serial"');
  });

  it('mengisi form dari data instalasi (bukan draf kosong)', () => {
    // Konteks harus diambil dari baris instalasi aktif.
    expect(src).toMatch(/locationLatitude:\s*active\?\.location_latitude/);
    expect(src).toMatch(/locationLongitude:\s*active\?\.location_longitude/);
    expect(src).toMatch(/customerName:\s*active\?\.customer_name/);
  });

  it('mengirim work_order_id + koordinat saat membuat aset', () => {
    expect(src).toMatch(/work_order_id:\s*active\.id/);
    expect(src).toContain('buildInstallationQuickAssetPayload');
  });

  it('mendukung ODP selain ONT/ONU (lewat komponen bersama)', () => {
    const panel = readFileSync(
      resolve(process.cwd(), 'src/lib/components/installations/InstallationQuickAssetPanel.svelte'),
      'utf8',
    );
    expect(panel).toMatch(/value:\s*'odp'/);
    expect(panel).toMatch(/value:\s*'ont'/);
    expect(panel).toMatch(/value:\s*'onu'/);
  });

  it('mengusulkan induk dari node FTTH yang sudah dipilih', () => {
    expect(src).toMatch(/selected_node_id/);
    expect(src).toContain('quickAssetParentFromNode');
  });

  it('punya tombol pemicu di UI', () => {
    expect(src).toContain('btn_create_asset');
    expect(src).toContain('openQuickAsset');
  });
});

describe('koordinat lokasi tidak dibuang lagi oleh tipe FE', () => {
  const types = read('lib/api/types.ts');

  it('InstallationWorkOrderView memuat location_latitude/longitude', () => {
    const blok = types.slice(types.indexOf('export interface InstallationWorkOrderView'));
    const akhir = blok.indexOf('\n}');
    const isi = blok.slice(0, akhir);
    expect(isi).toContain('location_latitude');
    expect(isi).toContain('location_longitude');
  });

  it('backend memang mengirim kolom itu (nama alias harus cocok)', () => {
    // Kalau alias di backend berubah, FE akan diam-diam dapat null lagi.
    const rs = readFileSync(
      resolve(
        process.cwd(),
        'src-tauri/src/services/customer_service/work_orders.rs',
      ),
      'utf8',
    );
    expect(rs).toContain('AS location_latitude');
    expect(rs).toContain('AS location_longitude');
  });
});

describe('rute lama benar-benar di-redirect ke v2', () => {
  it('sehingga perbaikan harus ada di v2, bukan hanya di halaman lama', () => {
    const redirect = read('lib/utils/legacyV2Redirect.ts');
    expect(redirect).toContain("'/admin/network/installations': '/v2/admin/network/installations'");
  });
});
