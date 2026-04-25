import { describe, expect, it } from 'vitest';

import {
  buildStraightLinkGeometryText,
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
