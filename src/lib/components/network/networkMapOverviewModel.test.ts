import { describe, expect, it } from 'vitest';

import { buildNetworkMapOverviewSearchGroups } from './networkMapOverviewModel';
import type { NMLink, NMNode, NMRouter, NMZone } from './networkMapUtils';

describe('network map overview model', () => {
  const nodes: NMNode[] = [
    {
      id: 'customer-1',
      name: 'Alpha Customer',
      node_type: 'customer_premise',
      status: 'active',
      lat: -6.2,
      lng: 106.8,
    },
    {
      id: 'service-1',
      name: 'Managed Switch',
      node_type: 'switch',
      status: 'active',
      lat: -6.21,
      lng: 106.81,
      metadata: { service_id: 'svc-1', service_name: 'Managed Switch' },
    },
    {
      id: 'node-1',
      name: 'Core POP',
      node_type: 'pop',
      status: 'maintenance',
      lat: -6.22,
      lng: 106.82,
    },
  ];

  const links: NMLink[] = [
    {
      id: 'link-1',
      name: 'Backhaul Ring',
      link_type: 'fiber',
      status: 'degraded',
      from_node_id: 'node-1',
      to_node_id: 'service-1',
      geometry: { type: 'LineString', coordinates: [] as [number, number][] },
    },
  ];

  const zones: NMZone[] = [
    {
      id: 'zone-1',
      name: 'Jakarta Selatan',
      zone_type: 'coverage',
      status: 'active',
      geometry: { type: 'Polygon', coordinates: [] as [number, number][][] },
    },
  ];

  const routers: NMRouter[] = [
    {
      id: 'router-1',
      name: 'Mikrotik Core',
      host: '10.0.0.1',
      port: 8728,
      is_online: true,
      enabled: true,
    },
  ];

  it('filters search groups based on quick mode', () => {
    const groups = buildNetworkMapOverviewSearchGroups({
      query: '',
      quickMode: 'field',
      nodes,
      links,
      zones,
      routers,
      customerRows: [nodes[0] as any],
      serviceRows: [nodes[1] as any],
    });

    expect(groups.map((group) => group.key)).toEqual(['customers', 'services', 'nodes', 'zones']);
  });

  it('keeps all groups when quick mode is all', () => {
    const groups = buildNetworkMapOverviewSearchGroups({
      query: '',
      quickMode: 'all',
      nodes,
      links,
      zones,
      routers,
      customerRows: [nodes[0] as any],
      serviceRows: [nodes[1] as any],
    });

    expect(groups.some((group) => group.key === 'customers')).toBe(true);
    expect(groups.some((group) => group.key === 'services')).toBe(true);
    expect(groups.some((group) => group.key === 'nodes')).toBe(true);
    expect(groups.some((group) => group.key === 'routers')).toBe(true);
    expect(groups.some((group) => group.key === 'links')).toBe(true);
    expect(groups.some((group) => group.key === 'zones')).toBe(true);
  });
});
