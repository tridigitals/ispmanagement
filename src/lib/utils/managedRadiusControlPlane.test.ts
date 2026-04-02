import { describe, expect, it } from 'vitest';

import {
  buildManagedRadiusRouterOsCli,
  filterManagedRadiusMappings,
  routerToOptionLabel,
} from './managedRadiusControlPlane';

describe('managed radius control plane helpers', () => {
  it('builds copy-ready RouterOS CLI with quoted host and secret', () => {
    expect(buildManagedRadiusRouterOsCli('radius.example.com', 's3cr"et', 1812, 1813)).toBe(
      '/radius add service=ppp address="radius.example.com" secret="s3cr\\"et" authentication-port=1812 accounting-port=1813 protocol=udp\n/ppp aaa set use-radius=yes accounting=yes',
    );
  });

  it('filters mappings by tenant, server, and search text', () => {
    const filtered = filterManagedRadiusMappings(
      [
        {
          id: 'map-1',
          tenant_id: 'tenant-1',
          tenant_name: 'Tenant One',
          radius_server_id: 'srv-1',
          server_name: 'Primary',
          router_id: 'r-1',
          router_name: 'Router A',
          nas_name: 'NAS-A',
          nas_ip_or_cidr: '10.0.0.1',
          shortname: null,
          shared_secret_masked: 'abcd••••••••wxyz',
          is_active: true,
          updated_at: '2026-04-02T00:00:00Z',
          radius_host: 'radius.example.com',
          auth_port: 1812,
          acct_port: 1813,
        },
        {
          id: 'map-2',
          tenant_id: 'tenant-2',
          tenant_name: 'Tenant Two',
          radius_server_id: 'srv-2',
          server_name: 'Backup',
          router_id: 'r-2',
          router_name: 'Router B',
          nas_name: 'NAS-B',
          nas_ip_or_cidr: '10.0.0.2',
          shortname: null,
          shared_secret_masked: 'efgh••••••••ijkl',
          is_active: true,
          updated_at: '2026-04-02T00:00:00Z',
          radius_host: 'radius-b.example.com',
          auth_port: 1812,
          acct_port: 1813,
        },
      ],
      {
        tenantId: 'tenant-1',
        serverId: 'srv-1',
        search: 'router a',
      },
    );

    expect(filtered).toHaveLength(1);
    expect(filtered[0]?.id).toBe('map-1');
  });

  it('formats router option labels with host fallback', () => {
    expect(routerToOptionLabel({ name: 'Main POP', host: '10.10.10.1' })).toBe(
      'Main POP (10.10.10.1)',
    );
    expect(routerToOptionLabel({ name: '', host: '10.10.10.2' })).toBe('10.10.10.2');
  });
});
