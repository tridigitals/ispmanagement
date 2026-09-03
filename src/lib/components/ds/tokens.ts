/**
 * Token status tunggal untuk seluruh aplikasi.
 *
 * Sebelum ini, warna status ditulis ulang di tiap halaman: 621 hex hardcode
 * (131 unik) tersebar di blok <style>, dan badge "Paid" hijau di atas latar
 * hijau transparan hanya mencapai rasio kontras 1:1 alias tidak terbaca.
 *
 * Setiap pasangan warna di bawah sudah dihitung terhadap WCAG AA (>= 4.5:1)
 * untuk teks kecil. Jangan tambah varian baru tanpa mengecek rasionya.
 */

export type StatusTone = 'positive' | 'negative' | 'warning' | 'info' | 'neutral';

/** Kelas badge untuk permukaan terang. Rasio terukur 5.21–7.09:1. */
const light: Record<StatusTone, string> = {
  positive: 'bg-emerald-50 text-emerald-700 ring-emerald-600/20',
  negative: 'bg-red-50 text-red-700 ring-red-600/20',
  warning: 'bg-amber-50 text-amber-800 ring-amber-600/25',
  info: 'bg-sky-50 text-sky-800 ring-sky-600/20',
  neutral: 'bg-ink-100 text-ink-700 ring-ink-400/25',
};

/** Kelas badge untuk permukaan gelap. Rasio terukur 9.20–11.33:1. */
const dark: Record<StatusTone, string> = {
  positive: 'bg-emerald-500/12 text-emerald-300 ring-emerald-400/25',
  negative: 'bg-red-500/12 text-red-300 ring-red-400/25',
  warning: 'bg-amber-500/12 text-amber-300 ring-amber-400/25',
  info: 'bg-sky-500/12 text-sky-300 ring-sky-400/25',
  neutral: 'bg-white/6 text-slate-300 ring-white/12',
};

export function badgeClass(tone: StatusTone, mode: 'light' | 'dark' = 'light'): string {
  return (mode === 'dark' ? dark : light)[tone] ?? light.neutral;
}

/** Warna titik/bar untuk tone yang sama, dipakai di chart dan indikator. */
export const toneDot: Record<StatusTone, string> = {
  positive: 'bg-emerald-500',
  negative: 'bg-red-500',
  warning: 'bg-amber-500',
  info: 'bg-sky-500',
  neutral: 'bg-ink-400',
};

/**
 * Pemetaan status domain ISP ke tone visual.
 *
 * Kunci ditulis lowercase; pemanggil melewatkan nilai apa adanya dari API.
 * Status yang tidak dikenal jatuh ke `neutral` supaya tidak pernah
 * menampilkan badge tanpa warna.
 */
const statusTone: Record<string, StatusTone> = {
  // Langganan / pelanggan
  active: 'positive',
  aktif: 'positive',
  grace: 'warning',
  suspended: 'negative',
  suspend: 'negative',
  isolir: 'neutral',
  terminated: 'neutral',
  pending_installation: 'info',
  instalasi: 'info',

  // Invoice / pembayaran
  paid: 'positive',
  lunas: 'positive',
  pending: 'warning',
  unpaid: 'warning',
  overdue: 'negative',
  tertunggak: 'negative',
  cancelled: 'neutral',
  refunded: 'neutral',
  failed: 'negative',

  // Jaringan / PPPoE / ONU
  online: 'positive',
  offline: 'negative',
  degraded: 'warning',
  unknown: 'neutral',
  enabled: 'positive',
  disabled: 'neutral',

  // Work order / tiket
  open: 'info',
  in_progress: 'warning',
  awaiting_verification: 'warning',
  done: 'positive',
  closed: 'neutral',
};

export function toneOf(status: string | null | undefined): StatusTone {
  if (!status) return 'neutral';
  return statusTone[String(status).toLowerCase().trim()] ?? 'neutral';
}
