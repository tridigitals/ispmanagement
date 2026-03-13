import { severityScore } from './wallboardUtils';

export type WallboardAlertRow = {
  id: string;
  router_id: string;
  severity: string;
  status: string;
  title: string;
  message: string;
  last_seen_at?: string | null;
  updated_at?: string | null;
};

export type WallboardIncidentRow = {
  id: string;
  router_id: string;
  interface_name?: string | null;
  incident_type: string;
  severity: string;
  status: string;
  title: string;
  message: string;
  last_seen_at?: string | null;
  updated_at?: string | null;
  resolved_at?: string | null;
};

export type WallboardNocRow = {
  id: string;
  name: string;
  identity?: string | null;
  is_online: boolean;
  latency_ms?: number | null;
};

export type WallboardTopIssue = {
  key: string;
  router_id: string;
  router_name: string;
  title: string;
  count: number;
  critical: number;
  warning: number;
  lastSeenMs: number;
};

export function sortAlerts(alerts: WallboardAlertRow[]) {
  const rank = (s?: string) => {
    const x = String(s || '').toLowerCase();
    if (x === 'critical') return 3;
    if (x === 'warning') return 2;
    return 1;
  };
  return [...alerts]
    .filter((a) => String(a.status || '').toLowerCase() !== 'resolved')
    .sort((a, b) => {
      const bySeverity = rank(b.severity) - rank(a.severity);
      if (bySeverity !== 0) return bySeverity;
      const ta = Date.parse(a.last_seen_at || a.updated_at || '');
      const tb = Date.parse(b.last_seen_at || b.updated_at || '');
      return (Number.isFinite(tb) ? tb : 0) - (Number.isFinite(ta) ? ta : 0);
    });
}

export function buildRouterAlertMap(sortedAlerts: WallboardAlertRow[]) {
  const map: Record<string, { total: number; critical: number; warning: number; ids: string[] }> = {};
  for (const a of sortedAlerts) {
    const rid = String(a.router_id || '');
    if (!rid) continue;
    map[rid] ||= { total: 0, critical: 0, warning: 0, ids: [] };
    map[rid].total += 1;
    const sev = String(a.severity || '').toLowerCase();
    if (sev === 'critical') map[rid].critical += 1;
    else if (sev === 'warning') map[rid].warning += 1;
    map[rid].ids.push(a.id);
  }
  return map;
}

export function buildAlertStats(sortedAlerts: WallboardAlertRow[]) {
  let critical = 0;
  let warning = 0;
  for (const a of sortedAlerts) {
    const sev = String(a.severity || '').toLowerCase();
    if (sev === 'critical') critical += 1;
    else if (sev === 'warning') warning += 1;
  }
  return { total: sortedAlerts.length, critical, warning };
}

export function getActiveIncidents(incidents: WallboardIncidentRow[]) {
  return incidents.filter((i) => {
    const status = String(i.status || '').toLowerCase();
    const resolvedAt = String(i.resolved_at || '').trim();
    return status !== 'resolved' && !resolvedAt;
  });
}

export function sortActiveIncidents(activeIncidents: WallboardIncidentRow[]) {
  return [...activeIncidents].sort((a, b) => {
    const sev = severityScore(b.severity) - severityScore(a.severity);
    if (sev !== 0) return sev;
    const ta = Date.parse(a.last_seen_at || a.updated_at || '');
    const tb = Date.parse(b.last_seen_at || b.updated_at || '');
    return (Number.isFinite(tb) ? tb : 0) - (Number.isFinite(ta) ? ta : 0);
  });
}

export function buildIncidentStats(activeIncidents: WallboardIncidentRow[]) {
  let critical = 0;
  let warning = 0;
  for (const i of activeIncidents) {
    const sev = String(i.severity || '').toLowerCase();
    if (sev === 'critical') critical += 1;
    else if (sev === 'warning') warning += 1;
  }
  return { total: activeIncidents.length, critical, warning };
}

export function filterVisibleAlerts(
  sortedAlerts: WallboardAlertRow[],
  alertSeverityFilter: 'all' | 'critical' | 'warning',
) {
  if (alertSeverityFilter === 'all') return sortedAlerts;
  return sortedAlerts.filter((a) => String(a.severity || '').toLowerCase() === alertSeverityFilter);
}

export function buildGlobalSummary(rows: WallboardNocRow[], incidentStats: { critical: number; warning: number }) {
  const total = rows.length;
  const online = rows.filter((r) => !!r.is_online).length;
  const offline = Math.max(0, total - online);
  const availability = total > 0 ? (online / total) * 100 : 0;
  const latencies = rows
    .filter((r) => !!r.is_online && Number.isFinite(r.latency_ms))
    .map((r) => Number(r.latency_ms));
  const avgLatencyMs = latencies.length
    ? latencies.reduce((a, b) => a + b, 0) / latencies.length
    : null;

  return {
    total,
    online,
    offline,
    availability,
    critical: incidentStats.critical,
    warning: incidentStats.warning,
    avgLatencyMs,
  };
}

export function buildTopIssues(args: {
  activeIncidents: WallboardIncidentRow[];
  rows: WallboardNocRow[];
}) {
  const oneHourAgo = Date.now() - 60 * 60 * 1000;
  const map = new Map<string, WallboardTopIssue>();
  for (const i of args.activeIncidents) {
    const tsRaw = i.last_seen_at || i.updated_at || '';
    const ts = Date.parse(tsRaw);
    if (!Number.isFinite(ts) || ts < oneHourAgo) continue;
    const routerId = String(i.router_id || '');
    if (!routerId) continue;
    const title = String(i.title || '').trim() || 'Incident';
    const key = `${routerId}::${title.toLowerCase()}`;
    const sev = String(i.severity || '').toLowerCase();

    if (!map.has(key)) {
      const rr = args.rows.find((row) => row.id === routerId);
      map.set(key, {
        key,
        router_id: routerId,
        router_name: rr?.identity || rr?.name || routerId,
        title,
        count: 0,
        critical: 0,
        warning: 0,
        lastSeenMs: ts,
      });
    }
    const cur = map.get(key)!;
    cur.count += 1;
    if (sev === 'critical') cur.critical += 1;
    else if (sev === 'warning') cur.warning += 1;
    cur.lastSeenMs = Math.max(cur.lastSeenMs, ts);
  }

  return Array.from(map.values())
    .sort((a, b) => {
      if (b.critical !== a.critical) return b.critical - a.critical;
      if (b.count !== a.count) return b.count - a.count;
      return b.lastSeenMs - a.lastSeenMs;
    })
    .slice(0, 5);
}

export function buildInsightsBadge(incidentStats: { critical: number; warning: number }) {
  const critical = incidentStats.critical;
  const warning = incidentStats.warning;
  const total = critical + warning;
  return {
    total,
    level: critical > 0 ? 'critical' : warning > 0 ? 'warning' : 'ok',
  } as const;
}
