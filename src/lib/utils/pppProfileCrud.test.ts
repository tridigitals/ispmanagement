import { describe, expect, it } from 'vitest';

import {
  getPppProfileCrudGateState,
  getPppProfileDeleteState,
  getPppProfileMutationErrorState,
  getPppProfileOnlyOneState,
  isPppProfileStaleTargetConflict,
  normalizePppProfilePayload,
} from './pppProfileCrud';

describe('ppp profile crud helpers', () => {
  it('blocks add and edit actions when no router is selected', () => {
    expect(getPppProfileCrudGateState('')).toEqual({
      blocked: true,
      reason: 'router_required',
    });

    expect(getPppProfileCrudGateState('router-1')).toEqual({
      blocked: false,
      reason: null,
    });
  });

  it('maps dependency counts into a blocked delete state when references exist', () => {
    expect(
      getPppProfileDeleteState({
        pppoe_accounts: 2,
        isp_package_router_mappings: 1,
      }),
    ).toEqual({
      status: 'blocked',
      blocked: true,
      allowed: false,
      dependencyCounts: {
        pppoe_accounts: 2,
        isp_package_router_mappings: 1,
      },
      totalDependencies: 3,
    });
  });

  it('maps empty dependencies into an allowed delete state', () => {
    expect(
      getPppProfileDeleteState({
        pppoe_accounts: 0,
        isp_package_router_mappings: 0,
      }),
    ).toEqual({
      status: 'allowed',
      blocked: false,
      allowed: true,
      dependencyCounts: {
        pppoe_accounts: 0,
        isp_package_router_mappings: 0,
      },
      totalDependencies: 0,
    });
  });

  it('distinguishes mirror sync failures from router write failures', () => {
    const mirrorSync = getPppProfileMutationErrorState('mirror_sync_failed');
    const routerWrite = getPppProfileMutationErrorState('router_write_failed');

    expect(mirrorSync).toEqual({
      code: 'mirror_sync_failed',
      tone: 'warning',
      title: 'RouterOS changed, but the local mirror could not be refreshed',
      message:
        'The PPP profile was written on the router, but the mirrored database cache could not be refreshed yet.',
    });

    expect(routerWrite).toEqual({
      code: 'router_write_failed',
      tone: 'error',
      title: 'RouterOS rejected the PPP profile change',
      message:
        'The PPP profile was not changed on the router. Fix the router error and try again.',
    });

    expect(mirrorSync.code).not.toBe(routerWrite.code);
    expect(mirrorSync.message).not.toBe(routerWrite.message);
  });

  it('detects stale target conflicts so the page can recover with sync', () => {
    expect(
      isPppProfileStaleTargetConflict(
        'PPP profile no longer exists on router. Sync from router before retrying.',
      ),
    ).toBe(true);
    expect(isPppProfileStaleTargetConflict('Router rejected PPP profile delete')).toBe(false);
  });

  it('normalizes payload strings and preserves only_one as a boolean', () => {
    expect(
      normalizePppProfilePayload({
        name: ' Basic-10M ',
        local_address: ' 10.10.10.1 ',
        remote_address: ' pool-basic ',
        rate_limit: ' 10M/10M ',
        dns_server: ' ',
        comment: ' standard ',
        only_one: true,
      }),
    ).toEqual({
      name: 'Basic-10M',
      local_address: '10.10.10.1',
      remote_address: 'pool-basic',
      rate_limit: '10M/10M',
      dns_server: null,
      comment: 'standard',
      only_one: true,
    });
  });

  it('maps only_one values into a stable enabled flag', () => {
    expect(getPppProfileOnlyOneState(true)).toEqual({ enabled: true });
    expect(getPppProfileOnlyOneState(false)).toEqual({ enabled: false });
    expect(getPppProfileOnlyOneState(null)).toEqual({ enabled: false });
  });
});
