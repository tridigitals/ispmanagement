import {
  groupNetworkMapSearchResults,
  type NetworkMapSearchResultGroup,
} from './networkMapInsights';
import type { NMLink, NMNode, NMRouter, NMZone } from './networkMapUtils';

type NetworkMapQuickMode = 'all' | 'issues' | 'customers' | 'services' | 'topology' | 'field';

export function buildNetworkMapOverviewSearchGroups(args: {
  query: string;
  quickMode: NetworkMapQuickMode;
  nodes: NMNode[];
  links: NMLink[];
  zones: NMZone[];
  routers: NMRouter[];
  customerRows: NMNode[];
  serviceRows: NMNode[];
}): NetworkMapSearchResultGroup[] {
  const groups = groupNetworkMapSearchResults({
    query: args.query,
    nodes: args.nodes,
    links: args.links,
    zones: args.zones,
    routers: args.routers,
    customerRows: args.customerRows,
    serviceRows: args.serviceRows,
  });

  if (args.quickMode === 'all') return groups;

  const allowedGroupKeys: Record<Exclude<NetworkMapQuickMode, 'all'>, string[]> = {
    issues: ['nodes', 'links', 'zones', 'routers'],
    customers: ['customers'],
    services: ['services'],
    topology: ['nodes', 'links', 'zones', 'routers'],
    field: ['customers', 'services', 'nodes', 'zones'],
  };

  const allowedKeys = allowedGroupKeys[args.quickMode] || [];
  return groups.filter((group) => allowedKeys.includes(group.key));
}

export function countNetworkMapSearchResults(groups: NetworkMapSearchResultGroup[]): number {
  return groups.reduce((total, group) => total + group.items.length, 0);
}
