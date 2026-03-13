type IncidentLike = {
  id: string;
  router_id: string;
  title: string;
};

export function incidentHrefById(tenantPrefix: string, id?: string | null) {
  const rid = String(id || '').trim();
  if (!rid) return `${tenantPrefix}/admin/network/incidents`;
  return `${tenantPrefix}/admin/network/incidents?incident=${encodeURIComponent(rid)}`;
}

export function incidentHrefForTopIssue(
  incidents: IncidentLike[],
  tenantPrefix: string,
  routerId: string,
  title: string,
) {
  const match = incidents.find((x) => x.router_id === routerId && x.title === title);
  return incidentHrefById(tenantPrefix, match?.id);
}
