import { describe, expect, it } from 'vitest';

import type { InstallationWorkOrderView, NetworkAssetListItem, UpdateNetworkAssetRequest } from '$lib/api/types';

import {
  buildInstallationAssetSyncUpdates,
  type InstallationAssetSyncUpdate,
} from './installationAssetSync';

function asset(overrides: Partial<NetworkAssetListItem>): NetworkAssetListItem {
  return {
    id: 'asset-1',
    tenant_id: 'tenant-1',
    asset_group: 'access_fiber',
    asset_type: 'ont',
    name: 'ONT-1',
    code: null,
    vendor: null,
    model: null,
    serial_number: null,
    status: 'available',
    customer_id: null,
    location_id: null,
    work_order_id: null,
    parent_asset_id: null,
    latitude: null,
    longitude: null,
    notes: null,
    metadata: {},
    created_at: '2026-05-12T00:00:00Z',
    updated_at: '2026-05-12T00:00:00Z',
    customer_name: null,
    location_label: null,
    work_order_status: null,
    parent_asset_name: null,
    ...overrides,
  };
}

function row(overrides: Partial<InstallationWorkOrderView>): InstallationWorkOrderView {
  return {
    id: 'wo-1',
    tenant_id: 'tenant-1',
    subscription_id: 'sub-1',
    invoice_id: null,
    customer_id: 'cust-1',
    location_id: 'loc-1',
    package_id: 'pkg-1',
    router_id: null,
    status: 'in_progress',
    assigned_to: 'tech-1',
    scheduled_at: null,
    completed_at: null,
    notes: null,
    created_at: '2026-05-12T00:00:00Z',
    updated_at: '2026-05-12T00:00:00Z',
    customer_name: 'Alpha',
    location_label: 'Home',
    package_name: 'FTTH 30M',
    package_provisioning_type: 'pppoe',
    router_name: null,
    assigned_to_name: null,
    assigned_to_email: null,
    assignment_id: null,
    assignment_status: null,
    subscription_status: 'pending_installation',
    subscription_starts_at: null,
    subscription_grace_until: null,
    has_customer_package_invoice: false,
    selected_zone_id: null,
    selected_zone_name: null,
    selected_node_id: null,
    selected_node_name: null,
    selected_node_score: null,
    path_node_ids: null,
    path_link_ids: null,
    ...overrides,
  };
}

describe('buildInstallationAssetSyncUpdates', () => {
  it('reserves the selected terminal asset and binds it to customer, location, work order, and parent asset', () => {
    const updates = buildInstallationAssetSyncUpdates({
      assets: [
        asset({ id: 'ont-1', asset_type: 'ont', status: 'available' }),
        asset({ id: 'odp-1', asset_type: 'odp', status: 'available' }),
      ],
      row: row({}),
      binding: {
        terminal_asset_id: 'ont-1',
        parent_asset_id: 'odp-1',
      },
    });

    expect(updates).toEqual<InstallationAssetSyncUpdate[]>([
      {
        id: 'ont-1',
        payload: {
          customer_id: 'cust-1',
          location_id: 'loc-1',
          work_order_id: 'wo-1',
          parent_asset_id: 'odp-1',
          status: 'reserved',
        } satisfies UpdateNetworkAssetRequest,
      },
      {
        id: 'odp-1',
        payload: {
          work_order_id: 'wo-1',
        } satisfies UpdateNetworkAssetRequest,
      },
    ]);
  });

  it('releases stale terminal and parent bindings from the same work order', () => {
    const updates = buildInstallationAssetSyncUpdates({
      assets: [
        asset({
          id: 'ont-selected',
          asset_type: 'ont',
          status: 'available',
        }),
        asset({
          id: 'ont-old',
          asset_type: 'ont',
          status: 'reserved',
          customer_id: 'cust-1',
          location_id: 'loc-1',
          work_order_id: 'wo-1',
          parent_asset_id: 'odp-old',
        }),
        asset({
          id: 'odp-selected',
          asset_type: 'odp',
          work_order_id: null,
        }),
        asset({
          id: 'odp-old',
          asset_type: 'odp',
          work_order_id: 'wo-1',
        }),
      ],
      row: row({}),
      binding: {
        terminal_asset_id: 'ont-selected',
        parent_asset_id: 'odp-selected',
      },
    });

    expect(updates).toEqual<InstallationAssetSyncUpdate[]>([
      {
        id: 'ont-selected',
        payload: {
          customer_id: 'cust-1',
          location_id: 'loc-1',
          work_order_id: 'wo-1',
          parent_asset_id: 'odp-selected',
          status: 'reserved',
        },
      },
      {
        id: 'ont-old',
        payload: {
          customer_id: null,
          location_id: null,
          work_order_id: null,
          parent_asset_id: null,
          status: 'available',
        },
      },
      {
        id: 'odp-selected',
        payload: {
          work_order_id: 'wo-1',
        },
      },
      {
        id: 'odp-old',
        payload: {
          work_order_id: null,
        },
      },
    ]);
  });

  it('avoids no-op updates when the selected binding is already in sync', () => {
    const updates = buildInstallationAssetSyncUpdates({
      assets: [
        asset({
          id: 'ont-1',
          asset_type: 'ont',
          status: 'reserved',
          customer_id: 'cust-1',
          location_id: 'loc-1',
          work_order_id: 'wo-1',
          parent_asset_id: 'odp-1',
        }),
        asset({
          id: 'odp-1',
          asset_type: 'odp',
          work_order_id: 'wo-1',
        }),
      ],
      row: row({}),
      binding: {
        terminal_asset_id: 'ont-1',
        parent_asset_id: 'odp-1',
      },
    });

    expect(updates).toEqual([]);
  });
});
