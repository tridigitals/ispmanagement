import { describe, expect, it } from 'vitest';

import {
  buildInstallationSubscriptionFallback,
  getInstallationInternetTestTargetOptions,
  getInstallationInternetTestTargetHint,
  resolveInstallationInternetTestRouterId,
  normalizeInstallationInternetTestTarget,
} from './installationInternetTest';

describe('installation internet test helpers', () => {
  it('offers router only when managed radius is not configured', () => {
    expect(
      getInstallationInternetTestTargetOptions({
        routerId: 'router-1',
        managedRadiusConfigured: false,
      }),
    ).toEqual([
      { value: 'router', label: 'Router', disabled: false },
      { value: 'managed_radius', label: 'RADIUS', disabled: true },
    ]);
  });

  it('offers router and radius when managed radius is configured', () => {
    expect(
      getInstallationInternetTestTargetOptions({
        routerId: 'router-1',
        managedRadiusConfigured: true,
      }),
    ).toEqual([
      { value: 'router', label: 'Router', disabled: false },
      { value: 'managed_radius', label: 'RADIUS', disabled: false },
    ]);
  });

  it('falls back to router when radius target is no longer available', () => {
    expect(
      normalizeInstallationInternetTestTarget(
        'managed_radius',
        getInstallationInternetTestTargetOptions({
          routerId: 'router-1',
          managedRadiusConfigured: false,
        }),
      ),
    ).toBe('router');
  });

  it('keeps managed radius selection when radius is available', () => {
    expect(
      normalizeInstallationInternetTestTarget(
        'managed_radius',
        getInstallationInternetTestTargetOptions({
          routerId: 'router-1',
          managedRadiusConfigured: true,
        }),
      ),
    ).toBe('managed_radius');
  });

  it('returns unavailable hint when managed radius is not configured', () => {
    expect(
      getInstallationInternetTestTargetHint({
        managedRadiusConfigured: false,
        managedRadiusLoadError: '',
      }),
    ).toBe('Managed RADIUS is not configured for this router yet');
  });

  it('returns plan hint when managed radius feature is gated', () => {
    expect(
      getInstallationInternetTestTargetHint({
        managedRadiusConfigured: false,
        managedRadiusLoadError: '',
        planUpgradeRequired: true,
      }),
    ).toBe('Managed RADIUS feature is not enabled for this tenant yet.');
  });

  it('returns assignment hint when tenant assignment is missing', () => {
    expect(
      getInstallationInternetTestTargetHint({
        managedRadiusConfigured: false,
        managedRadiusLoadError: '',
        tenantHasActiveAssignment: false,
        defaultServerAvailable: true,
      }),
    ).toBe('Managed RADIUS tenant assignment is not active yet.');
  });

  it('returns mapping hint when router mapping is missing', () => {
    expect(
      getInstallationInternetTestTargetHint({
        managedRadiusConfigured: false,
        managedRadiusLoadError: '',
        tenantHasActiveAssignment: true,
        canCreateMapping: true,
      }),
    ).toBe('Managed RADIUS NAS mapping for this router is not active yet.');
  });

  it('returns load hint when managed radius setup failed to load', () => {
    expect(
      getInstallationInternetTestTargetHint({
        managedRadiusConfigured: false,
        managedRadiusLoadError: 'Permission denied',
      }),
    ).toBe('Managed RADIUS setup could not be loaded. Check permissions or router setup.');
  });

  it('resolves router from package mapping when package has a single router mapping', () => {
    expect(
      resolveInstallationInternetTestRouterId({
        explicitRouterId: '',
        packageId: 'pkg-1',
        mappings: [
          { package_id: 'pkg-1', router_id: 'router-1' },
          { package_id: 'pkg-2', router_id: 'router-2' },
        ],
      }),
    ).toBe('router-1');
  });

  it('keeps router unresolved when package maps to multiple routers', () => {
    expect(
      resolveInstallationInternetTestRouterId({
        explicitRouterId: '',
        packageId: 'pkg-1',
        mappings: [
          { package_id: 'pkg-1', router_id: 'router-1' },
          { package_id: 'pkg-1', router_id: 'router-2' },
        ],
      }),
    ).toBe('');
  });

  it('builds a subscription fallback from work order fields for technician context', () => {
    expect(
      buildInstallationSubscriptionFallback({
        id: 'wo-1',
        tenant_id: 'tenant-1',
        subscription_id: 'sub-1',
        customer_id: 'customer-1',
        location_id: 'location-1',
        package_id: 'pkg-1',
        router_id: 'router-1',
        package_name: 'Home 20M',
        location_label: 'Main House',
        router_name: 'Core Router',
        subscription_status: 'pending_installation',
        subscription_grace_until: '2026-04-25T12:00:00Z',
        created_at: '2026-04-25T01:00:00Z',
        updated_at: '2026-04-25T02:00:00Z',
      }),
    ).toMatchObject({
      id: 'sub-1',
      tenant_id: 'tenant-1',
      customer_id: 'customer-1',
      location_id: 'location-1',
      package_id: 'pkg-1',
      router_id: 'router-1',
      package_name: 'Home 20M',
      location_label: 'Main House',
      router_name: 'Core Router',
      status: 'pending_installation',
      grace_until: '2026-04-25T12:00:00Z',
    });
  });

  it('does not build a subscription fallback when work order lacks a package', () => {
    expect(
      buildInstallationSubscriptionFallback({
        id: 'wo-1',
        tenant_id: 'tenant-1',
        subscription_id: 'sub-1',
        customer_id: 'customer-1',
        location_id: 'location-1',
        package_id: null,
        router_id: 'router-1',
        created_at: '2026-04-25T01:00:00Z',
        updated_at: '2026-04-25T02:00:00Z',
      }),
    ).toBeNull();
  });
});
