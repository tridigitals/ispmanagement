import { describe, expect, it } from 'vitest';

import { buildNetworkMapInsightCards, groupNetworkMapSearchResults } from './networkMapInsights';
import {
  countDegradedLinks,
  countImpactedServices,
  type NMNode,
  type NMRouter,
  type NMLink,
  type NMZone,
} from './networkMapUtils';

const technicianCapabilities = {
  canManageTopology: false,
  canReadCustomers: true,
  canReadWorkOrders: true,
  canReadNetworkNoc: false,
  canReadRouterInventory: true,
};

const manageTechnicianCapabilities = {
  canManageTopology: true,
  canReadCustomers: true,
  canReadWorkOrders: true,
  canReadNetworkNoc: false,
  canReadRouterInventory: true,
};

const nocCapabilities = {
  canManageTopology: false,
  canReadCustomers: false,
  canReadWorkOrders: false,
  canReadNetworkNoc: true,
  canReadRouterInventory: true,
};

const nodes: NMNode[] = [
  {
    id: 'node-1',
    name: 'Core POP',
    node_type: 'core',
    status: 'active',
    lat: -6.2,
    lng: 106.8,
    metadata: { service_count: 3 },
  },
  {
    id: 'node-2',
    name: 'Customer Site A',
    node_type: 'customer_premise',
    status: 'degraded',
    lat: -6.21,
    lng: 106.81,
    metadata: { impacted_services: 4 },
  },
  {
    id: 'node-3',
    name: 'Field Switch',
    node_type: 'switch',
    status: 'maintenance',
    lat: -6.22,
    lng: 106.82,
    metadata: { service_name: 'Metro Access' },
  },
];

const links: NMLink[] = [
  {
    id: 'link-1',
    name: 'Backbone Fiber',
    link_type: 'fiber',
    status: 'degraded',
    from_node_id: 'node-1',
    to_node_id: 'node-2',
    capacity_mbps: 1000,
    utilization_pct: 94,
    loss_db: 3.4,
    latency_ms: 52,
    geometry: {
      type: 'LineString',
      coordinates: [
        [106.8, -6.2],
        [106.81, -6.21],
      ],
    },
  },
];

const zones: NMZone[] = [
  {
    id: 'zone-1',
    name: 'North Cluster',
    zone_type: 'distribution',
    status: 'planning',
    geometry: {
      type: 'Polygon',
      coordinates: [
        [
          [106.79, -6.19],
          [106.83, -6.19],
          [106.83, -6.23],
          [106.79, -6.23],
          [106.79, -6.19],
        ],
      ],
    },
  },
];

const routers: Array<Partial<NMRouter>> = [
  { id: 'router-1', name: 'Edge Router', is_online: true, enabled: true },
  { id: 'router-2', name: 'Branch Router' },
];

describe('buildNetworkMapInsightCards', () => {
  it('orders manage-capable cards ahead of technician-style cards', () => {
    const cards = buildNetworkMapInsightCards({
      capabilities: manageTechnicianCapabilities,
      nodes,
      links,
      zones,
      routers,
      viewportMode: 'global',
    });

    expect(cards.map((card) => card.key).slice(0, 2)).toEqual(['field-work', 'impacted-services']);
  });

  it('orders technician cards around impacted services and field work', () => {
    const cards = buildNetworkMapInsightCards({
      capabilities: technicianCapabilities,
      nodes,
      links,
      zones,
      routers,
      viewportMode: 'global',
    });

    expect(cards.map((card) => card.key).slice(0, 2)).toEqual(['impacted-services', 'field-work']);
  });

  it('orders NOC cards around nodes at risk and degraded links', () => {
    const cards = buildNetworkMapInsightCards({
      capabilities: nocCapabilities,
      nodes,
      links,
      zones,
      routers,
      viewportMode: 'viewport',
    });

    expect(cards.map((card) => card.key).slice(0, 2)).toEqual(['nodes-at-risk', 'degraded-links']);
  });

  it('does not generate standalone router insight cards anymore', () => {
    const cards = buildNetworkMapInsightCards({
      capabilities: nocCapabilities,
      nodes: [],
      links: [],
      zones: [],
      routers: [{ id: 'router-3', name: 'Edge Router' }],
      viewportMode: 'viewport',
    });

    expect(cards).toEqual([]);
  });

  it('returns empty insight collections for empty topology data', () => {
    expect(
      buildNetworkMapInsightCards({
        capabilities: nocCapabilities,
        nodes: [],
        links: [],
        zones: [],
        routers: [],
        viewportMode: 'global',
      }),
    ).toEqual([]);

    expect(
      groupNetworkMapSearchResults({
        query: '',
        nodes: [],
        links: [],
        zones: [],
        routers: [],
        customerRows: [],
        serviceRows: [],
      }),
    ).toEqual([]);
  });
});

describe('network map utility counts', () => {
  it('uses explicit impacted service metadata instead of double-counting risky customer nodes', () => {
    expect(
      countImpactedServices([
        {
          id: 'customer-node',
          name: 'Customer Site A',
          node_type: 'customer_premise',
          status: 'degraded',
          lat: -6.2,
          lng: 106.8,
          metadata: { impacted_services: 4 },
        },
      ]),
    ).toBe(4);
  });

  it('ignores retired links when counting degraded links', () => {
    expect(
      countDegradedLinks([
        {
          id: 'link-retired',
          name: 'Old Backbone',
          link_type: 'fiber',
          status: 'retired',
          from_node_id: 'node-1',
          to_node_id: 'node-2',
          geometry: {
            type: 'LineString',
            coordinates: [
              [106.8, -6.2],
              [106.81, -6.21],
            ],
          },
        },
      ]),
    ).toBe(0);
  });
});

describe('groupNetworkMapSearchResults', () => {
  it('groups customer and service rows separately from general assets', () => {
    const groups = groupNetworkMapSearchResults({
      query: '',
      nodes,
      links,
      zones,
      routers,
      customerRows: [nodes[1]],
      serviceRows: [nodes[2]],
    });

    expect(groups.map((group) => group.key)).toEqual([
      'customers',
      'services',
      'nodes',
      'links',
      'zones',
      'routers',
    ]);
    expect(groups[0]?.items[0]).toMatchObject({ kind: 'customer', id: 'node-2' });
    expect(groups[1]?.items[0]).toMatchObject({ kind: 'service', id: 'node-3' });
  });
});
