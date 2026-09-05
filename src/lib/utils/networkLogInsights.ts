/**
 * Helper murni log jaringan v2 (gelombang 24b).
 *
 * Tone badge level log dulu berupa kelas CSS inline (`crit`/`warn`/
 * `debug`/`info`) di halaman legacy — kini pemetaan murni + tes.
 */
export type LogLevelTone = 'negative' | 'warning' | 'info' | 'neutral';

export function logLevelTone(level?: string | null): LogLevelTone {
  const x = String(level || '').toLowerCase();
  if (x === 'critical' || x === 'error') return 'negative';
  if (x === 'warning') return 'warning';
  if (x === 'debug') return 'neutral';
  return 'info';
}

export function logLevelLabel(level?: string | null): string {
  return String(level || 'info').toLowerCase();
}

/** Kunci filter log utk perbandingan "apakah filter berubah" (total refresh). */
export interface LogFilterKey {
  routerId: string;
  level: string;
  topic: string;
  q: string;
  month: string;
  year: string;
}

export function logFilterKey(f: LogFilterKey): string {
  return JSON.stringify(f);
}
