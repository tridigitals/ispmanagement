import { describe, expect, it } from 'vitest';

import { buildLinkGeometryDraftText } from './networkMapDrafts';
import type { NMNode } from './networkMapUtils';

const nodeRows: NMNode[] = [
  {
    id: 'asset-node-1',
    name: 'ODP Node',
    node_type: 'odp',
    status: 'active',
    lat: -7.2663,
    lng: 110.3837,
  },
];

describe('buildLinkGeometryDraftText', () => {
  it('prefers the visible source coordinate override for path drafts started from a topology asset marker', () => {
    const geometryText = buildLinkGeometryDraftText({
      linkPickDrawMode: 'path',
      nodeRows,
      linkForm: {
        name: 'ODP to Customer',
        link_type: 'fiber',
        status: 'up',
        from_node_id: 'asset-node-1',
        to_node_id: '',
        priority: '100',
        capacity_mbps: '',
        utilization_pct: '',
        loss_db: '',
        latency_ms: '',
        geometryText: '',
      },
      linkPathBendPoints: [[110.384, -7.2665]],
      sourceCoordOverride: [110.3840926, -7.2665442],
    } as any);

    expect(JSON.parse(geometryText)).toEqual({
      type: 'LineString',
      coordinates: [
        [110.3840926, -7.2665442],
        [110.384, -7.2665],
      ],
    });
  });
});
