/**
 * Helper murni untuk halaman Insiden Jaringan v2 (gelombang 19).
 *
 * Logika ini dulu terkubur di dalam komponen legacy (1.603 baris):
 * bobot severity, durasi terbuka, MTTA/MTTR, level SLA. Dipisah supaya
 * bisa dites dan dipakai konsisten antara daftar, kartu statistik, dan
 * modal detail.
 */

export interface IncidentLike {
  status: string;
  severity: string;
  first_seen_at?: string | null;
  last_seen_at?: string | null;
  acked_at?: string | null;
  resolved_at?: string | null;
  updated_at: string;
  is_auto_escalated?: boolean;
}

export const SEVERITY_ORDER = ['critical', 'warning', 'info'] as const;

export function severityWeight(severity: string): number {
  if (severity === 'critical') return 3;
  if (severity === 'warning') return 2;
  if (severity === 'info') return 1;
  return 0;
}

export function severityLabel(severity: string): string {
  if (severity === 'critical') return 'Kritis';
  if (severity === 'warning') return 'Peringatan';
  return 'Info';
}

export function statusLabel(status: string): string {
  if (status === 'open') return 'Terbuka';
  if (status === 'ack') return 'Diakui';
  if (status === 'in_progress') return 'Ditangani';
  if (status === 'resolved') return 'Selesai';
  return status;
}

/** Titik awal dihitung: first_seen_at fallback updated_at (sama seperti
 *  legacy, tapi sekarang diuji). */
export function incidentStartMs(row: IncidentLike): number {
  const raw = row.first_seen_at || row.updated_at || row.last_seen_at;
  if (!raw) return 0;
  const t = Date.parse(raw);
  return Number.isFinite(t) ? t : 0;
}

/** Lama insiden terbuka (ms). Resolved -> sampai resolved_at; aktif ->
 *  sampai `now` yang di-inject (deterministik untuk tes). */
export function incidentOpenMs(row: IncidentLike, now: number): number {
  const start = incidentStartMs(row);
  const end = row.resolved_at ? Date.parse(row.resolved_at) : now;
  if (!Number.isFinite(end) || end < start) return 0;
  return end - start;
}

/** Durasi compact: 45d 3j / 3j 12m / 45m / <1m */
export function formatDurationCompact(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return '—';
  const m = Math.floor(ms / 60000);
  if (m < 1) return '<1m';
  const d = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  const mm = m % 60;
  if (d > 0) return `${d}d ${h}j`;
  if (h > 0) return `${h}j ${mm}m`;
  return `${mm}m`;
}

/** Rata-rata menit sampai diakui (MTTA) dari insiden ber-acked_at.
 *  null bila tidak ada sampel. */
export function meanTimeToAck(rows: IncidentLike[], now: number): number | null {
  const samples: number[] = [];
  for (const r of rows) {
    if (!r.acked_at) continue;
    const ack = Date.parse(r.acked_at);
    const start = incidentStartMs(r);
    if (!Number.isFinite(ack) || ack < start) continue;
    samples.push(Math.round((ack - start) / 60000));
  }
  if (!samples.length) return null;
  return Math.round(samples.reduce((a, b) => a + b, 0) / samples.length);
}

/** Rata-rata menit sampai selesai (MTTR). */
export function meanTimeToResolve(rows: IncidentLike[]): number | null {
  const samples: number[] = [];
  for (const r of rows) {
    if (!r.resolved_at) continue;
    const end = Date.parse(r.resolved_at);
    const start = incidentStartMs(r);
    if (!Number.isFinite(end) || end < start) continue;
    samples.push(Math.round((end - start) / 60000));
  }
  if (!samples.length) return null;
  return Math.round(samples.reduce((a, b) => a + b, 0) / samples.length);
}

/** Level SLA berdasar lama terbuka. Ambang (menit) DI-INPUT dari setelan
 *  tenant (legacy membaca slaWarnMinutes/slaBreachMinutes dari API
 *  settings — helper tidak boleh hardcode kebijakan bisnis). */
export function slaLevel(
  row: IncidentLike,
  now: number,
  warnMinutes: number,
  breachMinutes: number,
): 'ok' | 'warn' | 'breach' {
  if (row.status === 'resolved' || row.resolved_at) return 'ok';
  const mins = incidentOpenMs(row, now) / 60000;
  if (mins >= breachMinutes) return 'breach';
  if (mins >= warnMinutes) return 'warn';
  return 'ok';
}

/** Ringkasan hitungan status untuk kartu statistik. */
export function incidentCounts(rows: IncidentLike[]): {
  open: number;
  ack: number;
  inProgress: number;
  resolved: number;
} {
  let open = 0;
  let ack = 0;
  let inProgress = 0;
  let resolved = 0;
  for (const r of rows) {
    if (r.status === 'open') open++;
    else if (r.status === 'ack') ack++;
    else if (r.status === 'in_progress') inProgress++;
    else if (r.status === 'resolved') resolved++;
  }
  return { open, ack, inProgress, resolved };
}

/** Pesan error backend insiden -> Indonesia ramah. */
export function friendlyIncidentError(raw: string | null | undefined): string {
  const msg = (raw ?? '').trim();
  if (!msg) return 'Terjadi kesalahan yang tidak diketahui.';
  const lower = msg.toLowerCase();
  if (lower.includes('not found')) return 'Insiden tidak ditemukan — mungkin sudah dihapus.';
  if (lower.includes('already resolved')) return 'Insiden ini sudah selesai.';
  if (lower.includes('not a member')) return 'Penanggung jawab harus anggota tenant ini.';
  if (lower.includes('incident_type is required')) return 'Tipe insiden wajib diisi.';
  if (lower.includes('database error') || lower.includes('internal server error')) {
    return 'Terjadi kesalahan di server. Coba lagi sebentar.';
  }
  return msg;
}
