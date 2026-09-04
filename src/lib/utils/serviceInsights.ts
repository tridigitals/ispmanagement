/**
 * Helper murni untuk halaman Layanan (paket ISP) v2.
 *
 * Dipisahkan dari komponen supaya bisa diuji tanpa DOM — sama seperti
 * announcementInsights/auditInsights. Logika ini dulunya terkubur di
 * ServicesDialogs.svelte (930 baris) dan +page.svelte legacy.
 */

export type ServiceType = 'internet_pppoe' | 'hotspot' | 'vpn';
export type ProvisioningType = 'pppoe' | 'dhcp_static';

export function normalizeServiceType(value?: string | null): ServiceType {
  const key = String(value || 'internet_pppoe').toLowerCase();
  if (key === 'hotspot') return 'hotspot';
  if (key === 'vpn') return 'vpn';
  return 'internet_pppoe';
}

export function normalizeProvisioningType(value?: string | null): ProvisioningType {
  return String(value || 'pppoe').toLowerCase() === 'dhcp_static' ? 'dhcp_static' : 'pppoe';
}

export function isInternetType(value?: string | null): boolean {
  return normalizeServiceType(value) === 'internet_pppoe';
}

export function isPppoeProvisioning(value?: string | null): boolean {
  return normalizeProvisioningType(value) === 'pppoe';
}

/** Pemetaan hanya masuk akal untuk Internet/PPPoE (ditegakkan juga di server). */
export function mappingAllowed(serviceType?: string | null, provisioningType?: string | null): boolean {
  return isInternetType(serviceType) && isPppoeProvisioning(provisioningType);
}

export function serviceTypeLabel(
  value?: string | null,
  provisioningType?: string | null,
): string {
  const key = String(value || 'internet_pppoe').toLowerCase();
  if (key === 'hotspot') return 'Hotspot';
  if (key === 'vpn') return 'VPN';
  return normalizeProvisioningType(provisioningType) === 'dhcp_static'
    ? 'Internet / DHCP Static'
    : 'Internet / PPPoE';
}

export function provisioningTypeLabel(value?: string | null): string {
  return normalizeProvisioningType(value) === 'dhcp_static' ? 'DHCP Static' : 'PPPoE';
}

export type ServiceTone = 'positive' | 'info' | 'neutral';
export function serviceTypeTone(value?: string | null): ServiceTone {
  const t = normalizeServiceType(value);
  if (t === 'internet_pppoe') return 'positive';
  if (t === 'hotspot') return 'info';
  return 'neutral';
}

/** Digit mata uang: IDR/JPY/KRW nol, sisanya dua. */
export function currencyDigits(code: string): number {
  const c = String(code || 'IDR').toUpperCase();
  return c === 'IDR' || c === 'JPY' || c === 'KRW' ? 0 : 2;
}

export function roundForCurrency(amount: number, currencyCode: string): number {
  const factor = Math.pow(10, currencyDigits(currencyCode));
  return Math.round(amount * factor) / factor;
}

/**
 * Konversi harga basis -> tampilan tenant. Tanpa kurs (null) atau mata uang
 * sama: tampilkan angka basis apa adanya (panggilan format memakai base).
 */
export function convertPrice(
  amountBase: number,
  baseCurrency: string,
  tenantCurrency: string,
  fxRate: number | null,
): { amount: number; currency: string } {
  const amount = Number(amountBase || 0);
  if (!amount) return { amount: 0, currency: tenantCurrency || baseCurrency };
  const tc = String(tenantCurrency || baseCurrency).toUpperCase();
  const bc = String(baseCurrency || 'IDR').toUpperCase();
  if (tc === bc || !fxRate) return { amount, currency: bc };
  return { amount: roundForCurrency(amount * fxRate, tc), currency: tc };
}

export interface PackageFormDraft {
  name: string;
  priceMonthly: number;
  priceYearly: number;
  yearlyEnabled: boolean;
}

/**
 * Validasi draft paket. Aturan SAMA dengan server (create/update_package):
 * nama wajib, bulanan > 0, tahunan > 0 saat toggle aktif. Server tetap
 * penjaga terakhir; ini hanya mencegah round-trip gagal.
 */
export function validatePackageDraft(d: PackageFormDraft): string[] {
  const errs: string[] = [];
  if (!d.name.trim()) errs.push('Nama paket wajib diisi.');
  if (!(Number(d.priceMonthly) > 0)) errs.push('Harga bulanan harus lebih dari 0.');
  if (d.yearlyEnabled && !(Number(d.priceYearly) > 0)) {
    errs.push('Harga tahunan harus lebih dari 0 saat opsi tahunan aktif.');
  }
  return errs;
}

/** Ringkasan pemakaian untuk tooltip/kolom: jumlah langganan aktif per paket. */
export function usageSummary(
  subscriptionsByPackage: Record<string, number>,
  packageId: string,
): number {
  return subscriptionsByPackage[packageId] ?? 0;
}

/**
 * Pesan error hapus dari server (guard referensi) -> instruksi yang jelas.
 * Server membalas "Package 'X' is still in use: 551 subscriptions, ...".
 */
export function friendlyDeleteError(message?: string | null): string {
  const m = String(message || '');
  if (/still in use/i.test(m)) {
    return m.replace(
      /is still in use: (.+)\. Move or cancel.*/i,
      'masih dipakai ($1). Pindahkan atau batalkan data terkait dulu.',
    );
  }
  return m || 'Gagal menghapus paket.';
}
