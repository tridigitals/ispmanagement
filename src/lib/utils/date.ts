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
export function timeAgo(date: string | Date): string {
  const now = new Date();
  const past = new Date(date);
  const diffMs = now.getTime() - past.getTime();
  const diffSec = Math.round(diffMs / 1000);
  const diffMin = Math.round(diffSec / 60);
  const diffHour = Math.round(diffMin / 60);
  const diffDay = Math.round(diffHour / 24);

  if (diffSec < 30) return 'Just now';
  if (diffSec < 60) return `${diffSec}s ago`;
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;
  if (diffDay < 30) return `${Math.floor(diffDay / 7)}w ago`;

  return past.toLocaleDateString();
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
