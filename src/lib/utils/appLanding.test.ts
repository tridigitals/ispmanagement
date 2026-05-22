import { describe, expect, it } from 'vitest';

import {
  canAccessCustomerDashboard,
  getDefaultTenantLandingPath,
  hasInternalAppAccess,
  type LandingUserLike,
} from './appLanding';

function makeUser(overrides: Partial<LandingUserLike> = {}): LandingUserLike {
  return {
    role: 'customer',
    is_super_admin: false,
    permissions: [],
    ...overrides,
  };
}

describe('app landing helpers', () => {
  it('treats customer portal users as dashboard-first', () => {
    const user = makeUser({
      permissions: ['customers:read_own', 'support:read'],
    });

    expect(hasInternalAppAccess(user)).toBe(false);
    expect(getDefaultTenantLandingPath(user, '/tenant-a')).toBe('/dashboard');
  });

  it('treats admin access permission as internal landing access', () => {
    const user = makeUser({
      permissions: ['admin:access'],
    });

    expect(hasInternalAppAccess(user)).toBe(true);
    expect(canAccessCustomerDashboard(user)).toBe(false);
    expect(getDefaultTenantLandingPath(user, '/tenant-a')).toBe('/admin');
  });

  it('treats granular internal permissions like technician access as admin landing access', () => {
    const user = makeUser({
      permissions: ['work_orders:read', 'pppoe:manage'],
    });

    expect(hasInternalAppAccess(user)).toBe(true);
    expect(canAccessCustomerDashboard(user)).toBe(false);
    expect(getDefaultTenantLandingPath(user, '/tenant-a')).toBe('/admin');
  });

  it('keeps superadmins on superadmin landing when there is no tenant prefix', () => {
    const user = makeUser({
      is_super_admin: true,
      permissions: ['*'],
    });

    expect(canAccessCustomerDashboard(user)).toBe(false);
    expect(getDefaultTenantLandingPath(user, '')).toBe('/admin');
  });

  it('keeps customer-only users eligible for the customer dashboard', () => {
    const user = makeUser({
      permissions: ['customers:read_own', 'support:read'],
    });

    expect(canAccessCustomerDashboard(user)).toBe(true);
  });
});
