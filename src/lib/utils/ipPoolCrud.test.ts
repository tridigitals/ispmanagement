import { describe, expect, it } from 'vitest';

import {
  getIpPoolCrudGateState,
  getIpPoolDeleteState,
  getIpPoolMutationErrorState,
  isIpPoolStaleTargetConflict,
} from './ipPoolCrud';

describe('ip pool crud helpers', () => {
  it('blocks add and edit actions when no router is selected', () => {
    expect(getIpPoolCrudGateState('')).toEqual({
      blocked: true,
      reason: 'router_required',
    });

    expect(getIpPoolCrudGateState('router-1')).toEqual({
      blocked: false,
      reason: null,
    });
  });

  it('maps dependency counts into a warning delete state when references exist', () => {
    expect(
      getIpPoolDeleteState({
        pppoe_accounts: 2,
        isp_package_router_mappings: 1,
      }),
    ).toEqual({
      status: 'warning',
      warning: true,
      allowed: true,
      dependencyCounts: {
        pppoe_accounts: 2,
        isp_package_router_mappings: 1,
      },
      totalDependencies: 3,
    });
  });

  it('maps empty dependencies into a clean delete state', () => {
    expect(
      getIpPoolDeleteState({
        pppoe_accounts: 0,
        isp_package_router_mappings: 0,
      }),
    ).toEqual({
      status: 'clean',
      warning: false,
      allowed: true,
      dependencyCounts: {
        pppoe_accounts: 0,
        isp_package_router_mappings: 0,
      },
      totalDependencies: 0,
    });
  });

  it('distinguishes mirror sync failures from router write failures', () => {
    const mirrorSync = getIpPoolMutationErrorState('mirror_sync_failed');
    const routerWrite = getIpPoolMutationErrorState('router_write_failed');

    expect(mirrorSync).toEqual({
      code: 'mirror_sync_failed',
      tone: 'warning',
      title: 'RouterOS changed, but the local mirror could not be refreshed',
      message: 'The IP pool was written on the router, but the mirrored database cache could not be refreshed yet.',
    });

    expect(routerWrite).toEqual({
      code: 'router_write_failed',
      tone: 'error',
      title: 'RouterOS rejected the IP pool change',
      message: 'The IP pool was not changed on the router. Fix the router error and try again.',
    });
  });

  it('detects stale target conflicts so the page can recover with sync', () => {
    expect(isIpPoolStaleTargetConflict('IP pool no longer exists on router. Sync from router before retrying.')).toBe(
      true,
    );
    expect(isIpPoolStaleTargetConflict('Router rejected IP pool delete')).toBe(false);
  });
});
