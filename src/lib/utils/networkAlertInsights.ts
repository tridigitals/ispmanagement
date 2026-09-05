/**
 * Helper murni alert jaringan v2 (gelombang 24b).
 *
 * Bobot severity, label tipe/severity, dan filter rentang tanggal dulu
 * inline di halaman legacy — kini pemetaan murni + tes.
 */
export type AlertRowLite = {
  status: string;
  severity: string;
  alert_type: string;
  last_seen_at: string;
};

export function alertSeverityWeight(severity: string): number {
  if (severity === 'critical') return 3;
  if (severity === 'warning') return 2;
  if (severity === 'info') return 1;
  return 0;
}

export function alertTypeLabel(tpe: string): string {
  if (tpe === 'offline') return 'Offline';
  if (tpe === 'cpu') return 'CPU';
  if (tpe === 'latency') return 'Latensi';
  return tpe;
}

export function alertSeverityLabel(sev: string): string {
  if (sev === 'critical') return 'Kritis';
  if (sev === 'warning') return 'Peringatan';
  return 'Info';
}

export function alertSeverityTone(sev: string): 'negative' | 'warning' | 'info' {
  if (sev === 'critical') return 'negative';
  if (sev === 'warning') return 'warning';
  return 'info';
}

export function alertStatusTone(status: string): 'positive' | 'info' | 'neutral' {
  if (status === 'resolved') return 'positive';
  if (status === 'ack') return 'info';
  return 'neutral';
}

export function filterAlertRows<T extends AlertRowLite>(
  rows: T[],
  f: { status: string; severity: string; type: string; from: string; to: string; sort: string },
): T[] {
  const list = rows.filter((row) => {
    if (f.status !== 'all' && row.status !== f.status) return false;
    if (f.severity !== 'all' && row.severity !== f.severity) return false;
    if (f.type !== 'all' && row.alert_type !== f.type) return false;
    const seenTs = new Date(row.last_seen_at).getTime();
    if (Number.isNaN(seenTs)) return false;
    if (f.from) {
      const fromTs = new Date(`${f.from}T00:00:00`).getTime();
      if (!Number.isNaN(fromTs) && seenTs < fromTs) return false;
    }
    if (f.to) {
      const toTs = new Date(`${f.to}T23:59:59.999`).getTime();
      if (!Number.isNaN(toTs) && seenTs > toTs) return false;
    }
    return true;
  });
  list.sort((a, b) => {
    const aSeen = new Date(a.last_seen_at).getTime() || 0;
    const bSeen = new Date(b.last_seen_at).getTime() || 0;
    if (f.sort === 'last_seen_asc') return aSeen - bSeen;
    if (f.sort === 'severity_desc') {
      const bySev = alertSeverityWeight(b.severity) - alertSeverityWeight(a.severity);
      if (bySev !== 0) return bySev;
    }
    return bSeen - aSeen;
  });
  return list;
}
