import { describe, expect, it } from 'vitest';

import { canAccessNetworkMap } from './adminNetworkAccess';

function canFromPermissions(permissions: string[]) {
  return (action: string, resource: string) =>
    permissions.includes('*') ||
    permissions.includes(`${resource}:*`) ||
    permissions.includes(`${resource}:${action}`);
}

describe('admin network access helpers', () => {
  it('allows technician map access through router inventory or work order permissions', () => {
    expect(canAccessNetworkMap(canFromPermissions(['router_inventory:read']))).toBe(true);
    expect(canAccessNetworkMap(canFromPermissions(['work_orders:read']))).toBe(true);
    expect(canAccessNetworkMap(canFromPermissions(['work_orders:manage']))).toBe(true);
  });

  it('keeps topology permissions valid for planner/admin map access', () => {
    expect(canAccessNetworkMap(canFromPermissions(['network_topology:read']))).toBe(true);
    expect(canAccessNetworkMap(canFromPermissions(['network_topology:manage']))).toBe(true);
  });

  it('denies users without operational network map context', () => {
    expect(canAccessNetworkMap(canFromPermissions(['support:read']))).toBe(false);
  });
});
