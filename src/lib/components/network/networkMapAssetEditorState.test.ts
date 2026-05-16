import { describe, expect, it } from 'vitest';

import { buildNetworkMapAssetEditorState } from './networkMapAssetEditorState';

describe('buildNetworkMapAssetEditorState', () => {
  it('builds an edit draft and detail draft from a topology asset row source item', () => {
    const result = buildNetworkMapAssetEditorState({
      id: 'asset-1',
      tenant_id: 'tenant-1',
      asset_group: 'access_fiber',
      asset_type: 'odp',
      name: 'ODP-1',
      code: 'ODP-001',
      vendor: 'FiberHome',
      model: 'ODP Box',
      serial_number: 'SN-1',
      status: 'available',
      customer_id: null,
      location_id: null,
      work_order_id: null,
      parent_asset_id: 'odc-1',
      latitude: -7.2665442,
      longitude: 110.3840926,
      notes: 'Near mosque',
      metadata: {
        total_port_capacity: '8',
        splitter_ratio: '1:8',
      },
      created_at: '2026-05-10T00:00:00Z',
      updated_at: '2026-05-10T00:00:00Z',
      customer_name: null,
      location_label: null,
      work_order_status: null,
      parent_asset_name: 'ODC-1',
    });

    expect(result.draft).toEqual({
      asset_type: 'odp',
      name: 'ODP-1',
      code: 'ODP-001',
      vendor: 'FiberHome',
      model: 'ODP Box',
      serial_number: 'SN-1',
      status: 'available',
      latitude: '-7.2665442',
      longitude: '110.3840926',
      notes: 'Near mosque',
    });
    expect(result.detailDraft).toMatchObject({
      total_port_capacity: '8',
      splitter_ratio: '1:8',
    });
  });
});
