import { describe, expect, it } from 'vitest';

import { getEnabledGlobalSearchProviderKeys } from './globalSearchProviders';
import type { GlobalSearchProviderContext } from './globalSearchModel';

function buildContext(overrides?: Partial<GlobalSearchProviderContext>): GlobalSearchProviderContext {
  return {
    can: () => false,
    isSuperAdmin: false,
    shellScope: 'admin',
    tenantPrefix: '',
    ...overrides,
  };
}

describe('globalSearchProviders', () => {
  it('enables admin-shell providers from RBAC capabilities', () => {
    const context = buildContext({
      can: (action, resource) =>
        (resource === 'customers' && action === 'read') ||
        (resource === 'router_inventory' && action === 'read') ||
        (resource === 'billing' && action === 'read') ||
        (resource === 'team' && action === 'read') ||
        (resource === 'support' && action === 'create'),
    });

    expect(getEnabledGlobalSearchProviderKeys(context)).toEqual([
      'customers',
      'routers',
      'invoices',
      'team-members',
      'support-tickets',
    ]);
  });

  it('enables only superadmin-shell providers on superadmin pages', () => {
    const context = buildContext({
      shellScope: 'superadmin',
      isSuperAdmin: true,
      can: () => true,
    });

    expect(getEnabledGlobalSearchProviderKeys(context)).toEqual(['tenants', 'superadmin-invoices']);
  });

  it('keeps admin-shell providers available for superadmin users when they are on admin pages', () => {
    const context = buildContext({
      shellScope: 'admin',
      isSuperAdmin: true,
      can: () => true,
    });

    expect(getEnabledGlobalSearchProviderKeys(context)).toEqual([
      'customers',
      'routers',
      'invoices',
      'team-members',
      'support-tickets',
    ]);
  });
});
