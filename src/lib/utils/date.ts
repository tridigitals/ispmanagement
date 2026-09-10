/**
 * Date Utilities
 */

export type DateFormatOptions = {
  locale?: string;
  timeZone?: string;
};

/**
 * Format date to relative time (e.g. "2 hours ago")
 */
export function timeAgo(date: string | Date, tt?: (k: string, v?: Record<string, string | number>) => string): string {
  const now = new Date();
  const past = new Date(date);
  if (Number.isNaN(past.getTime())) return '—';
  const diffMs = now.getTime() - past.getTime();
  // Jam backend meleset / data masa depan: jangan tampilkan tanggal aneh.
  if (diffMs < 0) return tt ? tt('common.time.just_now') : 'baru saja';
  const diffSec = Math.round(diffMs / 1000);
  const diffMin = Math.round(diffSec / 60);
  const diffHour = Math.round(diffMin / 60);
  const diffDay = Math.round(diffHour / 24);

  if (diffSec < 30) return tt ? tt('common.time.just_now') : 'baru saja';
  if (diffSec < 60) return tt ? tt('common.time.sec', { n: diffSec }) : `${diffSec} dtk lalu`;
  if (diffMin < 60) return tt ? tt('common.time.min', { n: diffMin }) : `${diffMin} mnt lalu`;
  if (diffHour < 24) return tt ? tt('common.time.hour', { n: diffHour }) : `${diffHour} jam lalu`;
  if (diffDay < 7) return tt ? tt('common.time.day', { n: diffDay }) : `${diffDay} hari lalu`;
  if (diffDay < 30) return tt ? tt('common.time.week', { n: Math.floor(diffDay / 7) }) : `${Math.floor(diffDay / 7)} mgg lalu`;

  return past.toLocaleDateString('id-ID', { day: 'numeric', month: 'short', year: 'numeric' });
}

/**
 * Format date to simple string
 */
export function formatDate(date: string | Date | number, opts: DateFormatOptions = {}): string {
  const dt = new Date(date);
  return new Intl.DateTimeFormat(opts.locale || undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    timeZone: opts.timeZone || undefined,
  }).format(dt);
}

export function formatTime(date: string | Date | number, opts: DateFormatOptions = {}): string {
  const dt = new Date(date);
  return new Intl.DateTimeFormat(opts.locale || undefined, {
    hour: '2-digit',
    minute: '2-digit',
    timeZone: opts.timeZone || undefined,
  }).format(dt);
}

export function formatDateTime(date: string | Date | number, opts: DateFormatOptions = {}): string {
  const dt = new Date(date);
  return new Intl.DateTimeFormat(opts.locale || undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    timeZone: opts.timeZone || undefined,
  }).format(dt);
}
