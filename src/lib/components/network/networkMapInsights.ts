import {
  countDegradedLinks,
  countImpactedServices,
  countNodesAtRisk,
  hasServiceMetadata,
  isCustomerNodeType,
  nodeTypeLabel,
  statusTone,
  summarizeZoneRisk,
  type NMLink,
  type NMNode,
  type NMRouter,
  type NMZone,
} from './networkMapUtils';

export type NetworkMapInsightCapabilities = {
  canManageTopology: boolean;
  canReadCustomers: boolean;
  canReadWorkOrders: boolean;
  canReadNetworkNoc: boolean;
  canReadRouterInventory: boolean;
};

export type NetworkMapInsightRole = 'technician' | 'noc' | 'manage' | 'viewer';
export type NetworkMapInsightViewportMode = 'global' | 'viewport';

export type NetworkMapInsightCardKey =
  | 'impacted-services'
  | 'field-work'
  | 'nodes-at-risk'
  | 'degraded-links'
  | 'zone-risk'
  | 'routers';

export type NetworkMapInsightCard = {
  key: NetworkMapInsightCardKey;
  label: string;
  value: number;
  detail: string;
  tone: 'ok' | 'warn' | 'muted';
  viewportMode: NetworkMapInsightViewportMode;
};

export type NetworkMapSearchResultItemKind =
  | 'customer'
  | 'service'
  | 'node'
  | 'link'
  | 'zone'
  | 'router';

export type NetworkMapSearchResultItem = {
  kind: NetworkMapSearchResultItemKind;
  id: string;
  label: string;
  subtitle: string;
  status: string;
  tone: 'ok' | 'warn' | 'muted';
};

export type NetworkMapSearchResultGroupKey =
  | 'customers'
  | 'services'
  | 'nodes'
  | 'links'
  | 'zones'
  | 'routers';

export type NetworkMapSearchResultGroup = {
  key: NetworkMapSearchResultGroupKey;
  label: string;
  items: NetworkMapSearchResultItem[];
};

function normalizeText(value: unknown) {
  return String(value ?? '')
    .trim()
    .toLowerCase();
}

function matchesQuery(item: { label: string; subtitle: string; status: string; kind: string }, query: string) {
  if (!query) return true;
  const haystack = `${item.label} ${item.subtitle} ${item.status} ${item.kind}`.toLowerCase();
  return haystack.includes(query);
}

function resolveInsightRole(
  capabilities?: Partial<NetworkMapInsightCapabilities> | null,
  explicitRole?: NetworkMapInsightRole | null,
): NetworkMapInsightRole {
  if (explicitRole) return explicitRole;
  if (!capabilities) return 'viewer';
  if (
    capabilities.canReadNetworkNoc &&
    !capabilities.canReadCustomers &&
    !capabilities.canReadWorkOrders &&
    !capabilities.canManageTopology
  ) {
    return 'noc';
  }
  if (capabilities.canReadCustomers || capabilities.canReadWorkOrders) return 'technician';
  if (capabilities.canManageTopology) return 'manage';
  return 'viewer';
}

function summarizeRouters(routers: Array<Partial<NMRouter>>) {
  const total = (routers || []).length;
  const online = (routers || []).filter((row) => row?.is_online === true).length;
  return {
    total,
    online,
    offline: Math.max(0, total - online),
  };
}

function buildInsightCard(args: {
  key: NetworkMapInsightCardKey;
  label: string;
  value: number;
  detail: string;
  tone: NetworkMapInsightCard['tone'];
  viewportMode: NetworkMapInsightViewportMode;
}) {
  return {
    key: args.key,
    label: args.label,
    value: args.value,
    detail: args.detail,
    tone: args.tone,
    viewportMode: args.viewportMode,
  } satisfies NetworkMapInsightCard;
}

export function buildNetworkMapInsightCards(args: {
  capabilities?: Partial<NetworkMapInsightCapabilities> | null;
  role?: NetworkMapInsightRole | null;
  nodes: NMNode[];
  links: NMLink[];
  zones: NMZone[];
  routers: Array<Partial<NMRouter>>;
  viewportMode?: NetworkMapInsightViewportMode;
}): NetworkMapInsightCard[] {
  const role = resolveInsightRole(args.capabilities, args.role);
  const viewportMode = args.viewportMode ?? 'global';
  const nodes = args.nodes || [];
  const links = args.links || [];
  const zones = args.zones || [];
  const routers = args.routers || [];

  if (!nodes.length && !links.length && !zones.length && !routers.length) {
    return [];
  }

  const impactedServices = countImpactedServices(nodes);
  const nodesAtRisk = countNodesAtRisk(nodes);
  const degradedLinks = countDegradedLinks(links);
  const zoneRisk = summarizeZoneRisk(zones);
  const routerSummary = summarizeRouters(routers);
  const fieldWork = nodesAtRisk + degradedLinks + zoneRisk.atRisk;

  const cardsByKey = new Map<NetworkMapInsightCardKey, NetworkMapInsightCard>();

  if (impactedServices > 0) {
    cardsByKey.set(
      'impacted-services',
      buildInsightCard({
        key: 'impacted-services',
        label: 'Impacted services',
        value: impactedServices,
        detail: `${impactedServices} affected service relationships`,
        tone: 'warn',
        viewportMode,
      }),
    );
  }

  if (fieldWork > 0) {
    cardsByKey.set(
      'field-work',
      buildInsightCard({
        key: 'field-work',
        label: 'Field work',
        value: fieldWork,
        detail: `${fieldWork} likely site actions`,
        tone: fieldWork > 3 ? 'warn' : 'muted',
        viewportMode,
      }),
    );
  }

  if (nodesAtRisk > 0) {
    cardsByKey.set(
      'nodes-at-risk',
      buildInsightCard({
        key: 'nodes-at-risk',
        label: 'Nodes at risk',
        value: nodesAtRisk,
        detail: `${nodesAtRisk} nodes need attention`,
        tone: nodesAtRisk > 3 ? 'warn' : 'muted',
        viewportMode,
      }),
    );
  }

  if (degradedLinks > 0) {
    cardsByKey.set(
      'degraded-links',
      buildInsightCard({
        key: 'degraded-links',
        label: 'Degraded links',
        value: degradedLinks,
        detail: `${degradedLinks} links have poor health`,
        tone: degradedLinks > 2 ? 'warn' : 'muted',
        viewportMode,
      }),
    );
  }

  if (zoneRisk.atRisk > 0) {
    cardsByKey.set(
      'zone-risk',
      buildInsightCard({
        key: 'zone-risk',
        label: 'Zone risk',
        value: zoneRisk.atRisk,
        detail: `${zoneRisk.atRisk} zones are in a risky state`,
        tone: zoneRisk.atRisk > 2 ? 'warn' : 'muted',
        viewportMode,
      }),
    );
  }

  if (routerSummary.total > 0) {
    cardsByKey.set(
      'routers',
      buildInsightCard({
        key: 'routers',
        label: 'Routers',
        value: routerSummary.total,
        detail:
          routerSummary.online > 0
            ? `${routerSummary.online} online / ${routerSummary.total} total`
            : `${routerSummary.total} total`,
        tone: routerSummary.online > 0 ? 'ok' : 'muted',
        viewportMode,
      }),
    );
  }

  const roleOrder: Record<NetworkMapInsightRole, NetworkMapInsightCardKey[]> = {
    technician: ['impacted-services', 'field-work', 'nodes-at-risk', 'degraded-links', 'zone-risk', 'routers'],
    noc: ['nodes-at-risk', 'degraded-links', 'zone-risk', 'routers', 'impacted-services', 'field-work'],
    manage: ['field-work', 'impacted-services', 'nodes-at-risk', 'degraded-links', 'zone-risk', 'routers'],
    viewer: ['nodes-at-risk', 'degraded-links', 'zone-risk', 'routers', 'impacted-services', 'field-work'],
  };

  return roleOrder[role]
    .map((key) => cardsByKey.get(key))
    .filter((card): card is NetworkMapInsightCard => !!card);
}

function buildNodeSearchItem(row: NMNode): NetworkMapSearchResultItem {
  const label = String(row.name || row.id || 'Node').trim();
  const subtitle = nodeTypeLabel(row.node_type);
  return {
    kind: isCustomerNodeType(row.node_type) ? 'customer' : hasServiceMetadata(row) ? 'service' : 'node',
    id: row.id,
    label,
    subtitle,
    status: String(row.status || ''),
    tone: statusTone(row.status),
  };
}

function buildLinkSearchItem(row: NMLink): NetworkMapSearchResultItem {
  const label = String(row.name || row.id || 'Link').trim();
  const subtitle = `${String(row.from_node_id || '-')} -> ${String(row.to_node_id || '-')}`;
  return {
    kind: 'link',
    id: row.id,
    label,
    subtitle,
    status: String(row.status || ''),
    tone: countDegradedLinks([row]) > 0 ? 'warn' : 'ok',
  };
}

function buildZoneSearchItem(row: NMZone): NetworkMapSearchResultItem {
  return {
    kind: 'zone',
    id: row.id,
    label: String(row.name || row.id || 'Zone').trim(),
    subtitle: String(row.zone_type || '-'),
    status: String(row.status || ''),
    tone: statusTone(row.status),
  };
}

function buildRouterSearchItem(row: Partial<NMRouter>): NetworkMapSearchResultItem {
  const label = String(row.identity || row.name || row.id || 'Router').trim();
  const subtitle = [row.host, row.port].filter((part) => part != null && String(part).trim() !== '').join(':');
  return {
    kind: 'router',
    id: String(row.id || label),
    label,
    subtitle: subtitle || 'Router inventory',
    status: row.is_online === true ? 'online' : row.is_online === false ? 'offline' : '',
    tone: row.is_online === true ? 'ok' : 'muted',
  };
}

export function groupNetworkMapSearchResults(args: {
  query: string;
  nodes: NMNode[];
  links: NMLink[];
  zones: NMZone[];
  routers: Array<Partial<NMRouter>>;
  customerRows?: NMNode[];
  serviceRows?: NMNode[];
}): NetworkMapSearchResultGroup[] {
  const query = normalizeText(args.query);
  const customerRows = args.customerRows ?? (args.nodes || []).filter((row) => isCustomerNodeType(row.node_type));
  const serviceRows = args.serviceRows ?? (args.nodes || []).filter((row) => hasServiceMetadata(row));
  const excludedIds = new Set([...customerRows, ...serviceRows].map((row) => row.id));
  const generalNodeRows = (args.nodes || []).filter((row) => !excludedIds.has(row.id));

  const groups: Array<NetworkMapSearchResultGroup> = [
    {
      key: 'customers',
      label: 'Customers',
      items: customerRows
        .map(buildNodeSearchItem)
        .filter((item) => item.kind === 'customer' && matchesQuery(item, query)),
    },
    {
      key: 'services',
      label: 'Services',
      items: serviceRows
        .map(buildNodeSearchItem)
        .filter((item) => item.kind === 'service' && matchesQuery(item, query)),
    },
    {
      key: 'nodes',
      label: 'Nodes',
      items: generalNodeRows
        .map(buildNodeSearchItem)
        .filter((item) => item.kind === 'node' && matchesQuery(item, query)),
    },
    {
      key: 'links',
      label: 'Links',
      items: (args.links || []).map(buildLinkSearchItem).filter((item) => matchesQuery(item, query)),
    },
    {
      key: 'zones',
      label: 'Zones',
      items: (args.zones || []).map(buildZoneSearchItem).filter((item) => matchesQuery(item, query)),
    },
    {
      key: 'routers',
      label: 'Routers',
      items: (args.routers || []).map(buildRouterSearchItem).filter((item) => matchesQuery(item, query)),
    },
  ];

  return groups.filter((group) => group.items.length > 0);
}
