import { describe, expect, it } from 'vitest';

import {
  buildStraightLinkGeometryText,
  resolveCanonicalCustomerNodeId,
  resolveLinkGeometryTextForSubmit,
  type NetworkMapLinkForm,
} from './networkMapLinkPicking';
import type { NMNode } from './networkMapUtils';

const nodes: NMNode[] = [
  {
    id: 'node-a',
    name: 'POP A',
    node_type: 'pop',
    status: 'active',
    lat: -6.2,
    lng: 106.8,
  },
  {
    id: 'node-b',
    name: 'ODP B',
    node_type: 'odp',
    status: 'active',
    lat: -6.25,
    lng: 106.85,
  },
];

function makeLinkForm(geometryText: string): NetworkMapLinkForm {
  return {
    name: 'POP A to ODP B',
    link_type: 'fiber',
    status: 'up',
    from_node_id: 'node-a',
    to_node_id: 'node-b',
    priority: '100',
    capacity_mbps: '',
    utilization_pct: '',
    loss_db: '',
    latency_ms: '',
    geometryText,
  };
}

describe('resolveLinkGeometryTextForSubmit', () => {
  it('keeps an existing drawn path when it has coordinates', () => {
    const drawnPath = JSON.stringify({
      type: 'LineString',
      coordinates: [
        [106.8, -6.2],
        [106.82, -6.22],
        [106.85, -6.25],
      ],
    });

    expect(resolveLinkGeometryTextForSubmit(makeLinkForm(drawnPath), nodes)).toBe(drawnPath);
  });

  it('falls back to a straight line when hidden geometry is empty', () => {
    const emptyGeometry = JSON.stringify({ type: 'LineString', coordinates: [] });
    const resolved = resolveLinkGeometryTextForSubmit(makeLinkForm(emptyGeometry), nodes);

    expect(resolved).toBe(buildStraightLinkGeometryText(nodes, 'node-a', 'node-b'));
  });
});

describe('resolveCanonicalCustomerNodeId', () => {
  it('prefers the visible deduped customer marker node for the same location', () => {
    const nodeRows: NMNode[] = [
      {
        id: 'customer-duplicate-a',
        name: 'Handono - Lokasi Utama',
        node_type: 'customer_premise',
        status: 'active',
        lat: -7.264948,
        lng: 110.383801,
        metadata: {
          customer_id: 'cust-1',
          location_id: 'loc-1',
        },
      },
      {
        id: 'customer-duplicate-b',
        name: 'Handono - Lokasi Utama',
        node_type: 'customer_premise',
        status: 'active',
        lat: -7.264948,
        lng: 110.383801,
        metadata: {
          customer_id: 'cust-1',
          location_id: 'loc-1',
        },
      },
    ];

    const customerRows: NMNode[] = [
      {
        ...nodeRows[1],
      },
    ];

    expect(resolveCanonicalCustomerNodeId('customer-duplicate-a', nodeRows, customerRows)).toBe(
      'customer-duplicate-b',
    );
  });
});
