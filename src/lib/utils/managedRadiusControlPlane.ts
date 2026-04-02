import type { SuperadminManagedRadiusMapping } from '$lib/api/types';

export type ManagedRadiusTabId = 'servers' | 'assignments' | 'mappings' | 'users';

type ManagedRadiusTabCounts = Record<ManagedRadiusTabId, number>;

type RouterLike = {
  name?: string | null;
  host?: string | null;
};

type MappingFilter = {
  tenantId?: string;
  serverId?: string;
  search?: string;
};

function normalized(value: string | null | undefined): string {
  return String(value ?? '')
    .trim()
    .toLowerCase();
}

function routerosQuote(value: string): string {
  return `"${value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
}

export function buildManagedRadiusRouterOsCli(
  radiusHost: string,
  sharedSecret: string,
  authPort: number,
  acctPort: number,
): string {
  return [
    `/radius add service=ppp address=${routerosQuote(radiusHost)} secret=${routerosQuote(sharedSecret)} authentication-port=${authPort} accounting-port=${acctPort} protocol=udp`,
    '/ppp aaa set use-radius=yes accounting=yes',
  ].join('\n');
}

export function filterManagedRadiusMappings(
  mappings: SuperadminManagedRadiusMapping[],
  filters: MappingFilter,
): SuperadminManagedRadiusMapping[] {
  const query = normalized(filters.search);

  return mappings.filter((mapping) => {
    const matchesTenant = !filters.tenantId || mapping.tenant_id === filters.tenantId;
    const matchesServer = !filters.serverId || mapping.radius_server_id === filters.serverId;
    const matchesSearch =
      !query ||
      [
        mapping.tenant_name,
        mapping.server_name,
        mapping.router_name,
        mapping.nas_name,
        mapping.nas_ip_or_cidr,
        mapping.shortname,
        mapping.radius_host,
      ].some((value) => normalized(value).includes(query));

    return matchesTenant && matchesServer && matchesSearch;
  });
}

export function buildManagedRadiusTabs(
  counts: ManagedRadiusTabCounts,
  activeTab: ManagedRadiusTabId,
): Array<{ id: ManagedRadiusTabId; count: number; active: boolean }> {
  return (['servers', 'assignments', 'mappings', 'users'] as ManagedRadiusTabId[]).map((id) => ({
    id,
    count: counts[id],
    active: id === activeTab,
  }));
}

export function routerToOptionLabel(router: RouterLike): string {
  const name = String(router.name ?? '').trim();
  const host = String(router.host ?? '').trim();

  if (name && host) return `${name} (${host})`;
  if (name) return name;
  if (host) return host;
  return 'Router';
}
