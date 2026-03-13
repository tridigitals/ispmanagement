import type { LayoutPreset } from '$lib/constants/wallboard';

export type TrendInfo = {
  dir: 'up' | 'down' | 'flat';
  deltaPct: number;
};

export type IncidentKind =
  | 'critical'
  | 'warning'
  | 'ack'
  | 'mute'
  | 'unmute'
  | 'poll_error'
  | 'recovered';

export function formatBps(bps: number | null | undefined, naLabel: string) {
  if (bps == null) return naLabel;
  const abs = Math.abs(bps);
  const units = ['bps', 'Kbps', 'Mbps', 'Gbps'];
  let u = 0;
  let v = abs;
  while (v >= 1000 && u < units.length - 1) {
    v /= 1000;
    u++;
  }
  const s = `${v >= 10 || u === 0 ? v.toFixed(0) : v.toFixed(1)} ${units[u]}`;
  return bps < 0 ? `-${s}` : s;
}

export function formatLatency(ms: number | null | undefined, naLabel: string) {
  if (ms == null || !Number.isFinite(ms)) return naLabel;
  const v = Number(ms);
  if (v < 1000) return `${Math.round(v)} ms`;
  return `${(v / 1000).toFixed(2)} s`;
}

export function maintenanceRemaining(until?: string | null) {
  const raw = String(until || '').trim();
  if (!raw) return null;
  const end = Date.parse(raw);
  if (!Number.isFinite(end)) return null;
  const diffMs = end - Date.now();
  if (diffMs <= 0) return null;
  const totalMin = Math.ceil(diffMs / 60_000);
  if (totalMin < 60) return `${totalMin}m`;
  const hh = Math.floor(totalMin / 60);
  const mm = totalMin % 60;
  return mm > 0 ? `${hh}h ${mm}m` : `${hh}h`;
}

export function parseMetricTs(ts?: string | null): number | null {
  const raw = String(ts || '').trim();
  if (!raw) return null;

  let ms = Date.parse(raw);
  if (Number.isFinite(ms)) return ms;

  let normalized = raw.includes(' ') && !raw.includes('T') ? raw.replace(' ', 'T') : raw;
  normalized = normalized.replace(/\.(\d{3})\d+(Z|[+-]\d{2}:?\d{2})$/, '.$1$2');
  normalized = normalized.replace(/\.(\d{3})\d+$/, '.$1');
  ms = Date.parse(normalized);
  if (Number.isFinite(ms)) return ms;

  if (!/[zZ]|[+-]\d{2}:?\d{2}$/.test(normalized)) {
    ms = Date.parse(`${normalized}Z`);
    if (Number.isFinite(ms)) return ms;
  }
  return null;
}

export function formatMetricTs(ts: string | null | undefined) {
  const ms = parseMetricTs(ts);
  if (!Number.isFinite(ms)) return '—';
  return new Date(ms).toLocaleString();
}

export function peakBps(list: number[]) {
  if (!list.length) return null;
  return Math.max(...list);
}

export function avgBps(list: number[]) {
  if (!list.length) return null;
  return Math.round(list.reduce((a, b) => a + b, 0) / list.length);
}

export function severityScore(s?: string) {
  const sev = String(s || '').toLowerCase();
  if (sev === 'critical') return 3;
  if (sev === 'warning') return 2;
  return 1;
}

export function calcTrend(list: number[]): TrendInfo {
  const points = list.filter((v) => Number.isFinite(v));
  if (points.length < 10) return { dir: 'flat', deltaPct: 0 };

  const win = Math.max(3, Math.min(6, Math.floor(points.length / 2)));
  const cur = points.slice(-win);
  const prev = points.slice(-(win * 2), -win);
  if (!cur.length || !prev.length) return { dir: 'flat', deltaPct: 0 };

  const avg = (arr: number[]) => arr.reduce((a, b) => a + b, 0) / Math.max(1, arr.length);
  const curAvg = avg(cur);
  const prevAvg = avg(prev);
  const base = Math.max(1, prevAvg);
  const deltaPct = ((curAvg - prevAvg) / base) * 100;

  if (Math.abs(deltaPct) < 5) return { dir: 'flat', deltaPct: 0 };
  return { dir: deltaPct > 0 ? 'up' : 'down', deltaPct };
}

export function trendBadgeText(
  ti: TrendInfo,
  stableLabel: string,
) {
  if (ti.dir === 'flat') return stableLabel;
  const pct = Math.abs(ti.deltaPct);
  const num = pct >= 10 ? Math.round(pct).toString() : pct.toFixed(1);
  return `${ti.dir === 'up' ? '↑' : '↓'} ${num}%`;
}

export function trendLabel(
  ti: TrendInfo,
  labels: { up: string; down: string; stable: string },
) {
  if (ti.dir === 'up') return labels.up;
  if (ti.dir === 'down') return labels.down;
  return labels.stable;
}

export function slotCountForLayout(p: LayoutPreset) {
  switch (p) {
    case '2x2':
      return 4;
    case '3x2':
      return 6;
    case '3x3':
      return 9;
    case '4x3':
      return 12;
  }
}

export function colsForLayout(p: LayoutPreset) {
  switch (p) {
    case '2x2':
      return 2;
    case '3x2':
      return 3;
    case '3x3':
      return 3;
    case '4x3':
      return 4;
  }
}

export function rowsForLayout(p: LayoutPreset) {
  switch (p) {
    case '2x2':
      return 2;
    case '3x2':
      return 2;
    case '3x3':
      return 3;
    case '4x3':
      return 3;
  }
}

export function formatIncidentTs(ms: number) {
  return new Date(ms).toLocaleString();
}

export function kindClass(kind: IncidentKind) {
  if (kind === 'critical' || kind === 'poll_error') return 'critical';
  if (kind === 'warning' || kind === 'mute') return 'warning';
  return 'ok';
}

export function kindLabel(kind: IncidentKind) {
  if (kind === 'critical') return 'Critical';
  if (kind === 'warning') return 'Warning';
  if (kind === 'ack') return 'Ack';
  if (kind === 'mute') return 'Mute';
  if (kind === 'unmute') return 'Unmute';
  if (kind === 'poll_error') return 'Poll Error';
  return 'Recovered';
}
