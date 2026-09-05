/**
 * Helper murni NOC v2 (gelombang 24b).
 *
 * Skor kesehatan, filter risiko, dan format throughput dulu inline di
 * halaman legacy — kini pemetaan murni + tes.
 */
export type NocRow = {
  id: string;
  name: string;
  is_online: boolean;
  cpu_load?: number | null;
  latency_ms?: number | null;
  maintenance_until?: string | null;
};

export type NocThresholds = { cpuRisk: number; cpuHot: number; latRisk: number; latHot: number };

export function nocInMaintenance(r: NocRow, now = Date.now()): boolean {
  if (!r.maintenance_until) return false;
  const t = new Date(r.maintenance_until).getTime();
  return Number.isFinite(t) && t > now;
}

export function nocHealthScore(r: NocRow, th: NocThresholds, now = Date.now()): number {
  if (nocInMaintenance(r, now)) return -1;
  if (!r.is_online) return 1000;
  const cpu = r.cpu_load ?? 0;
  const lat = r.latency_ms ?? 0;
  return Math.max(0, cpu - th.cpuRisk) + Math.max(0, Math.round((lat - th.latRisk) / 10));
}

export function nocIsHot(r: NocRow, th: NocThresholds, now = Date.now()): boolean {
  if (nocInMaintenance(r, now)) return false;
  return !r.is_online || (r.cpu_load ?? 0) >= th.cpuHot || (r.latency_ms ?? 0) >= th.latHot;
}

export function nocMatchesRisk(
  r: NocRow,
  risk: 'all' | 'hot' | 'latency' | 'cpu',
  th: NocThresholds,
  now = Date.now(),
): boolean {
  if (risk === 'all') return true;
  if (risk === 'hot') return nocIsHot(r, th, now);
  if (nocInMaintenance(r, now)) return false; // diredam
  const cpu = r.cpu_load ?? 0;
  const lat = r.latency_ms ?? 0;
  if (risk === 'latency') return lat >= th.latRisk;
  return cpu >= th.cpuRisk;
}

export function nocMemoryPct(total?: number | null, free?: number | null): number | null {
  if (!total || total <= 0 || free == null) return null;
  return Math.max(0, Math.min(100, Math.round(((total - free) / total) * 100)));
}

export function nocFormatBps(bps?: number | null): string {
  if (bps == null) return '—';
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
