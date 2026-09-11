/**
 * Sumber kebenaran "kanvas terang vs gelap" per rute — dipakai app.html
 * (inline copy, harus dijalankan sebelum Svelte compile) dan layout Svelte.
 *
 * Terang: seluruh v2 (kecuali wallboard NOC) + halaman auth publik.
 * Gelap: pay, install, superadmin, legacy (app), dan wallboard.
 */
export function isBrightCanvasPath(pathname: string): boolean {
  if (pathname === '/v2/admin/network/noc/wallboard') return false;
  if (pathname.startsWith('/v2/')) return true;
  return /^\/(login|register|forgot-password|verify-email|unauthorized|maintenance)(\/|$)/.test(pathname);
}
