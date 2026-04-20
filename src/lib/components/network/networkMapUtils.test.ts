import { describe, expect, it } from 'vitest';

import {
  buildLinkPopupModel,
  buildNodePopupModel,
  buildRouterPopupModel,
  buildRouterPopupModelFromNode,
  buildServicePopupModel,
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
      },
    };

    const model = buildServicePopupModel(serviceNode);

    expect(model.kicker).toBe('Service');
    expect(model.contextText).toBe('Active • PPPoE • Account ready');
    expect(model.title).toContain('Internet 20 Mbps');
    expect(model.subtitle).toContain('Budi Santoso');
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

  it('builds link popups with health and transport metrics', () => {
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

    expect(model.kicker).toBe('Link');
    expect(model.contextText).toContain('fiber');
    expect(model.summaryItems.map((item) => item.label)).toEqual(
      expect.arrayContaining(['Health', 'Capacity', 'Latency']),
    );
    expect(model.actions.map((action) => action.key)).toEqual(['delete']);
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
      ]),
    );
    expect(model.actions.map((action) => action.key)).toEqual(['open-router']);
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
