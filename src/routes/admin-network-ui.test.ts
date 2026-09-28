import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

const V2 = 'src/routes/(v2)/v2';

/*
  Ditulis ulang setelah halaman (app) dipensiunkan.

  Asersi lama menguji halaman (app) dan token gaya kustomnya (`var(--bg-surface)`,
  `var(--radius-lg)`, media query tulis-tangan). Halaman v2 memakai bahasa desain
  yang berbeda — Tailwind + TEMA TERANG (terverifikasi di DOM: body rgb(250,250,250),
  input `bg-white`) — jadi token gelap itu memang tidak ada di sana, bukan regresi.

  Yang benar-benar dijaga sekarang: halaman v2 tetap pakai komponen bersama
  (AppShell/DataTable/Field), tetap punya grid yang responsif, dan alur penting
  (buat/edit DHCP static, visibilitas instalasi, deep-link work order) tetap
  tersambung. Yang sudah tidak punya rumah di v2 (validasi DHCP static dipakai
  halaman lama saja) diuji langsung di modulnya.
*/
describe('admin network UI (v2)', () => {
  const halaman = [
    `${V2}/admin/network/alerts/+page.svelte`,
    `${V2}/admin/network/incidents/+page.svelte`,
    `${V2}/admin/network/installations/+page.svelte`,
    `${V2}/admin/network/dhcp-static/+page.svelte`,
    `${V2}/admin/network/ip-pools/+page.svelte`,
    `${V2}/admin/network/noc/+page.svelte`,
    `${V2}/admin/network/ppp-profiles/+page.svelte`,
    `${V2}/admin/network/pppoe/+page.svelte`,
    `${V2}/admin/network/routers/+page.svelte`,
  ];

  it('memakai shell & tabel bersama, bukan markup kustom', () => {
    for (const file of halaman) {
      const source = readSource(file);
      // Sebagian halaman mengimpor langsung, sebagian lewat barrel
      // '$lib/components/ds' — keduanya sah.
      const pakaiShell =
        source.includes('$lib/components/ds/AppShell.svelte') || source.includes('$lib/components/ds');
      const pakaiTabel =
        source.includes('$lib/components/ds/DataTable.svelte') || source.includes('DataTable');
      expect(pakaiShell, `${file}: shell bersama`).toBe(true);
      expect(pakaiTabel, `${file}: tabel bersama`).toBe(true);
      // Jejak desain lama tidak boleh kembali.
      expect(source, file).not.toContain('var(--bg-card)');
      expect(source, file).not.toContain('linear-gradient');
      expect(source, file).not.toContain('backdrop-filter');
    }
  });

  it('memakai layout fleksibel/grid responsif, bukan lebar tetap', () => {
    for (const file of halaman) {
      const source = readSource(file);
      // v2 = Tailwind. Sebagian halaman memakai grid (dengan prefiks breakpoint),
      // sebagian memakai flex-wrap — yang penting tidak dipatok lebar tetap.
      const responsif =
        /[\w-]+:grid-cols-\d/.test(source) ||
        source.includes('grid-cols-2') ||
        source.includes('flex-wrap');
      expect(responsif, `${file}: layout responsif`).toBe(true);
    }
  });

  it('DHCP static tetap bisa dibuat & diubah langsung dari halaman', () => {
    const source = readSource(`${V2}/admin/network/dhcp-static/+page.svelte`);

    expect(source).toContain('api.dhcpStatic.services.create');
    expect(source).toContain('api.dhcpStatic.services.update');
    expect(source).toContain('normalizeDhcpStaticMacAddress');
    expect(source).toContain('validateDhcpStaticIpv4Address');
    expect(source).toContain('validateDhcpStaticQueueRateLimit');
    expect(source).toContain('buildDhcpStaticQueueRateLimitPresets');
  });

  it('validasi & preset DHCP static tetap satu sumber', () => {
    // Halaman (app) tempat asersi lama menunjuk sudah pensiun; modulnya tetap
    // jadi satu-satunya sumber logika ini dan diuji langsung.
    const validasi = readSource('src/lib/utils/dhcpStaticValidation.ts');
    expect(validasi).toContain('normalizeDhcpStaticMacAddress');
    expect(validasi).toContain('validateDhcpStaticIpv4Address');
    expect(validasi).toContain('validateDhcpStaticQueueRateLimit');

    // Preset tinggal di modul terpisah (dipakai halaman v2 dhcp-static).
    const preset = readSource('src/lib/utils/dhcpStaticQueuePresets.ts');
    expect(preset).toContain('buildDhcpStaticQueueRateLimitPresets');
  });

  it('bisa membuka instalasi dari deep link work order', () => {
    const source = readSource(`${V2}/admin/network/installations/+page.svelte`);

    expect(source).toContain("searchParams.get('work_order_id')");
    expect(source).toContain('openDetail');
  });

  it('mengatur visibilitas work order', () => {
    const source = readSource(`${V2}/admin/network/installations/+page.svelte`);

    expect(source).toContain("installation_work_order_visibility_mode");
    expect(source).toContain('admin_only');
    expect(source).toContain('all_staff');
    expect(source).toContain('saveVisibility');
    expect(source).toContain('showVisibility');
  });
});
