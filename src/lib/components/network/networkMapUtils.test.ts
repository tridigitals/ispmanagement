import { describe, expect, it } from 'vitest';

import {
  buildLinkPopupModel,
  buildNodePopupModel,
  buildRouterPopupModel,
  buildRouterPopupModelFromNode,
  buildServicePopupModel,
  customersToFeatureCollection,
  getCustomerNodeIconId,
  getCustomerPppoeVisualState,
  manualNodeTypeOptions,
  nodesToFeatureCollection,
  type NMLink,
  type NMNode,
  type NMRouter,
} from './networkMapUtils';

describe('network map popup models', () => {
  it('builds odp popups with operational distribution details and no impact action', () => {
    const node: NMNode = {
      id: 'node-risky-1',
      name: 'ODP-17 Taman Sari',
      node_type: 'odp',
      status: 'degraded',
      lat: -6.2,
      lng: 106.8,
      metadata: {
        service_count: 12,
        splitter_count: 3,
        zone_name: 'Cluster Timur',
        parent_node_name: 'ODC-Salatiga-01',
      },
    };

    const model = buildNodePopupModel(node);

    expect(model.kicker).toBe('ODP');
    expect(model.contextText).toContain('distribution');
    expect(model.summaryItems.map((item) => item.label)).toEqual(
      expect.arrayContaining(['Services', 'Splitters']),
    );
    expect(model.detailPairs).toEqual(
      expect.arrayContaining([
        { label: 'Zone', value: 'Cluster Timur' },
        { label: 'Upstream', value: 'ODC-Salatiga-01' },
      ]),
    );
    expect(model.actions.map((action) => action.key)).toEqual(['connect', 'edit']);
  });

  it('hides legacy FTTH distribution nodes from the generic node marker layer', () => {
    const fc = nodesToFeatureCollection([
      {
        id: 'splitter-node-1',
        name: 'Legacy Splitter Node',
        node_type: 'splitter',
        status: 'active',
        lat: -7.2,
        lng: 110.3,
        metadata: {},
      },
      {
        id: 'router-node-1',
        name: 'Edge Router',
        node_type: 'router',
        status: 'active',
        lat: -7.21,
        lng: 110.31,
        metadata: {},
      },
    ]);

    expect(fc.features.map((feature) => feature.properties?.id)).toEqual(['router-node-1']);
  });

  it('keeps manual node picker focused on non-ftth topology node types', () => {
    expect(manualNodeTypeOptions.map((option) => option.value)).toEqual([
      'core',
      'pop',
      'router',
      'switch',
      'tower',
      'ap',
      'junction',
      'customer_premise',
    ]);
  });

  it('builds customer service popups with package and account details', () => {
    const serviceNode: NMNode = {
      id: 'svc-node-1',
      name: 'Service 20 Mbps',
      node_type: 'customer_premise',
      status: 'active',
      lat: -6.21,
      lng: 106.82,
      metadata: {
        service_id: 'svc-20',
        customer_id: 'cust-20',
        service_name: 'Internet 20 Mbps',
        service_type: 'pppoe',
        service_label: 'PPPoE Bronze',
        customer_name: 'Budi Santoso',
        package_name: 'Bronze 20 Mbps',
        pppoe_username: 'budi-pppoe',
        subscription_status: 'active',
        pppoe_visual_state: 'connected',
      },
    };

    const model = buildServicePopupModel(serviceNode);

    expect(model.kicker).toBe('Service');
    expect(model.contextText).toContain('online on Mikrotik');
    expect(model.title).toContain('Internet 20 Mbps');
    expect(model.subtitle).toContain('Budi Santoso');
    expect(model.statusText).toBe('active');
    expect(model.statusChips).toEqual([
      expect.objectContaining({ label: 'Subscription', value: 'Active', tone: 'ok' }),
      expect.objectContaining({ label: 'Mikrotik PPP', value: 'PPP Online', tone: 'ok' }),
    ]);
    expect(model.detailPairs).toEqual(
      expect.arrayContaining([
        { label: 'Package', value: 'Bronze 20 Mbps' },
        { label: 'Service', value: 'pppoe' },
        { label: 'Status', value: 'active' },
      ]),
    );
    expect(model.summaryItems).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ label: 'Customer', value: 'Budi Santoso' }),
        expect.objectContaining({ label: 'Account', value: 'budi-pppoe' }),
      ]),
    );
    expect(model.actions.map((action) => action.key)).toEqual([
      'open-customer',
      'open-service',
      'connect',
    ]);
    expect(model.actions[0]).toEqual(
      expect.objectContaining({ key: 'open-customer', label: 'Customer', tone: 'primary' }),
    );
    expect(model.actions[1]).toEqual(
      expect.objectContaining({ key: 'open-service', label: 'Service', tone: 'secondary' }),
    );
  });

  it('builds customer service popups with a red Mikrotik PPP state when account is offline', () => {
    const serviceNode: NMNode = {
      id: 'svc-node-2',
      name: 'Service 50 Mbps',
      node_type: 'customer_premise',
      status: 'active',
      lat: -6.21,
      lng: 106.82,
      metadata: {
        service_id: 'svc-50',
        customer_id: 'cust-50',
        service_name: 'Internet 50 Mbps',
        service_type: 'internet_pppoe',
        customer_name: 'Siti Aminah',
        package_name: 'Gold 50 Mbps',
        pppoe_username: 'siti-pppoe',
        subscription_status: 'active',
        pppoe_visual_state: 'disconnected',
      },
    };

    const model = buildServicePopupModel(serviceNode);

    expect(model.contextText).toContain('no active PPP session');
    expect(model.statusChips).toEqual([
      expect.objectContaining({ label: 'Subscription', value: 'Active', tone: 'ok' }),
      expect.objectContaining({ label: 'Mikrotik PPP', value: 'PPP Offline', tone: 'danger' }),
    ]);
    expect(model.summaryItems).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ label: 'Account', value: 'siti-pppoe', tone: 'danger' }),
      ]),
    );
    expect(model.actions.map((action) => action.key)).toEqual([
      'open-service',
      'open-customer',
      'connect',
    ]);
    expect(model.actions[0]).toEqual(
      expect.objectContaining({ key: 'open-service', label: 'Service', tone: 'primary' }),
    );
  });

  it('builds customer service popups with a muted Mikrotik PPP state when account is missing', () => {
    const serviceNode: NMNode = {
      id: 'svc-node-3',
      name: 'Service 10 Mbps',
      node_type: 'customer_premise',
      status: 'pending_installation',
      lat: -6.21,
      lng: 106.82,
      metadata: {
        service_id: 'svc-10',
        customer_id: 'cust-10',
        service_name: 'Starter 10 Mbps',
        service_type: 'internet_pppoe',
        customer_name: 'Nur Hidayah',
        package_name: 'Starter 10 Mbps',
        subscription_status: 'pending_installation',
      },
    };

    const model = buildServicePopupModel(serviceNode);

    expect(model.contextText).toContain('not been provisioned');
    expect(model.statusChips).toEqual([
      expect.objectContaining({
        label: 'Subscription',
        value: 'Pending Installation',
        tone: 'warn',
      }),
      expect.objectContaining({ label: 'Mikrotik PPP', value: 'PPP Belum Ada', tone: 'muted' }),
    ]);
    expect(model.summaryItems).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          label: 'Account',
          value: 'Belum ada akun PPP',
          tone: 'muted',
        }),
      ]),
    );
    expect(model.actions.map((action) => action.key)).toEqual([
      'open-service',
      'open-customer',
      'connect',
    ]);
    expect(model.actions[0]).toEqual(
      expect.objectContaining({ key: 'open-service', label: 'Service', tone: 'primary' }),
    );
  });

  it('builds compact link popups with only essential health and transport context', () => {
    const link: NMLink = {
      id: 'link-1',
      name: 'Backhaul Ungaran',
      link_type: 'fiber',
      status: 'degraded',
      from_node_id: 'ODC-1',
      to_node_id: 'ODP-7',
      capacity_mbps: 1000,
      utilization_pct: 72,
      latency_ms: 14,
      loss_db: 1.2,
      geometry: {
        type: 'LineString',
        coordinates: [
          [110.4, -7.1],
          [110.41, -7.11],
        ],
      },
    };

    const model = buildLinkPopupModel(link);

    expect(model.variant).toBe('network-link');
    expect(model.kicker).toBe('Link');
    expect(model.contextText).toContain('fiber');
    expect(model.summaryItems.map((item) => item.label)).toEqual(
      expect.arrayContaining(['Health', 'Capacity']),
    );
    expect(model.summaryItems.map((item) => item.label)).not.toContain('Latency');
    expect(model.detailPairs).toEqual([]);
    expect(model.actions.map((action) => action.key)).toEqual(['edit', 'delete']);
    expect(model.actions[0]).toEqual(
      expect.objectContaining({ key: 'edit', label: 'Edit', tone: 'primary' }),
    );
  });

  it('builds router popups with connection and version details', () => {
    const router: NMRouter = {
      id: 'router-1',
      name: 'Main Router',
      identity: 'RTR-SALATIGA-01',
      host: '10.10.10.1',
      port: 8728,
      is_online: true,
      enabled: true,
      ros_version: '7.16.1',
      latency_ms: 5,
    };

    const model = buildRouterPopupModel(router);

    expect(model.kicker).toBe('Mikrotik');
    expect(model.title).toBe('RTR-SALATIGA-01');
    expect(model.contextText.toLowerCase()).toContain('mikrotik');
    expect(model.detailPairs).toEqual(
      expect.arrayContaining([
        { label: 'Endpoint', value: '10.10.10.1:8728' },
        { label: 'RouterOS', value: '7.16.1' },
      ]),
    );
    expect(model.summaryItems).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ label: 'Connectivity', value: 'Live' }),
        expect.objectContaining({ label: 'Access', value: 'Enabled' }),
        expect.objectContaining({ label: 'Address', value: '10.10.10.1' }),
      ]),
    );
    expect(model.actions.map((action) => action.key)).toEqual(['connect', 'open-router']);
  });

  it('builds synced router node popups from live router inventory status', () => {
    const node: NMNode = {
      id: 'node-router-1',
      name: 'Solikin',
      node_type: 'router',
      status: 'inactive',
      lat: -6.2,
      lng: 106.8,
      metadata: {
        asset_source: 'mikrotik_router',
        asset_type: 'mikrotik_router',
        asset_id: 'router-live-1',
        zone_name: 'Semarang Barat',
      },
    };
    const router: NMRouter = {
      id: 'router-live-1',
      name: 'Solikin',
      identity: 'SOLIKIN-EDGE',
      host: '10.10.10.10',
      port: 8728,
      is_online: true,
      enabled: true,
      ros_version: '7.18.2',
      latency_ms: 3,
    };

    const model = buildRouterPopupModelFromNode(node, router);

    expect(model.kicker).toBe('Mikrotik');
    expect(model.statusText).toBe('online');
    expect(model.tone).toBe('ok');
    expect(model.title).toBe('SOLIKIN-EDGE');
    expect(model.summaryItems).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ label: 'Connectivity', value: 'Live' }),
        expect.objectContaining({ label: 'Latency', value: '3 ms' }),
      ]),
    );
    expect(model.detailPairs).toEqual(
      expect.arrayContaining([
        { label: 'Endpoint', value: '10.10.10.10:8728' },
        { label: 'Source', value: 'Router map' },
        { label: 'Zone', value: 'Semarang Barat' },
      ]),
    );
  });
});

describe('network map customer visual state helpers', () => {
  it('maps customer PPPoE visual state into feature properties and icon ids', () => {
    const row: NMNode = {
      id: 'cust-node-1',
      name: 'Budi - Rumah',
      node_type: 'customer_premise',
      status: 'active',
      lat: -6.2,
      lng: 106.8,
      metadata: {
        pppoe_visual_state: 'connected',
      },
    };

    expect(getCustomerPppoeVisualState(row)).toBe('connected');
    expect(getCustomerNodeIconId('connected')).toBe('nm-node-icon-customer-connected');

    const fc = customersToFeatureCollection([row]);
    expect(fc.features[0]?.properties).toEqual(
      expect.objectContaining({
        pppoe_visual_state: 'connected',
      }),
    );
  });

  it('falls back to neutral for missing or unknown visual state', () => {
    expect(getCustomerPppoeVisualState({ metadata: {} })).toBe('neutral');
    expect(getCustomerNodeIconId('unknown' as any)).toBe('nm-node-icon-customer-neutral');
  });
});
