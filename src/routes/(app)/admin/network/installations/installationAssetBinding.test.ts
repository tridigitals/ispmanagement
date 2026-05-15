import { describe, expect, it } from 'vitest';

import type { InstallationWorkOrderView, NetworkAssetListItem } from '$lib/api/types';

import {
  buildEmptyInstallationAssetBinding,
  buildInstallationParentAssetOptions,
  buildInstallationTerminalAssetOptions,
  resolveInstallationAssetBinding,
  validateInstallationAssetBinding,
} from './installationAssetBinding';

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
    serial_number: 'SN-1',
    status: 'available',
    customer_id: null,
    location_id: null,
    work_order_id: null,
    parent_asset_id: null,
    latitude: null,
    longitude: null,
    notes: null,
    metadata: {},
    created_at: '2026-05-11T00:00:00Z',
    updated_at: '2026-05-11T00:00:00Z',
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
    created_at: '2026-05-11T00:00:00Z',
    updated_at: '2026-05-11T00:00:00Z',
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

describe('installationAssetBinding', () => {
  it('builds terminal options from available or same-customer ONT/ONU assets', () => {
    const options = buildInstallationTerminalAssetOptions(
      [
        asset({ id: 'ont-free', name: 'ONT Free', asset_type: 'ont', customer_id: null }),
        asset({ id: 'onu-customer', name: 'ONU Customer', asset_type: 'onu', customer_id: 'cust-1' }),
        asset({ id: 'ont-other', name: 'ONT Other', asset_type: 'ont', customer_id: 'cust-2' }),
        asset({ id: 'router-1', name: 'Router', asset_type: 'router' }),
        asset({ id: 'ont-bad', name: 'ONT Bad', status: 'faulty' }),
      ],
      row({}),
    );

    expect(options.map((item) => item.value)).toEqual(['ont-free', 'onu-customer']);
  });

  it('builds parent options from FTTH upstream assets only', () => {
    const options = buildInstallationParentAssetOptions([
      asset({
        id: 'odp-1',
        name: 'ODP-1',
        asset_type: 'odp',
        metadata: { total_port_capacity: '8' },
      }),
      asset({ id: 'fat-1', name: 'FAT-1', asset_type: 'fat' }),
      asset({ id: 'switch-1', name: 'Switch-1', asset_type: 'switch' }),
      asset({
        id: 'ont-1',
        name: 'ONT-1',
        asset_type: 'ont',
        parent_asset_id: 'odp-1',
        status: 'installed',
      }),
    ]);

    expect(options.map((item) => item.value)).toEqual(['fat-1', 'odp-1']);
    expect(options.find((item) => item.value === 'odp-1')?.label).toContain('1/8 used');
  });

  it('resolves currently bound terminal and parent assets from work-order-linked assets', () => {
    expect(
      resolveInstallationAssetBinding(
        [
          asset({ id: 'ont-1', asset_type: 'ont', work_order_id: 'wo-1' }),
          asset({ id: 'odp-1', asset_type: 'odp', work_order_id: 'wo-1' }),
        ],
        'wo-1',
      ),
    ).toEqual({
      terminal_asset_id: 'ont-1',
      parent_asset_id: 'odp-1',
    });
  });

  it('requires terminal ONT/ONU asset before completion', () => {
    expect(
      validateInstallationAssetBinding(row({ status: 'in_progress' }), buildEmptyInstallationAssetBinding()),
    ).toBe('Select ONT/ONU asset before completion.');

    expect(
      validateInstallationAssetBinding(row({ status: 'in_progress' }), {
        terminal_asset_id: 'ont-1',
        parent_asset_id: '',
      }),
    ).toBeNull();
  });
});
