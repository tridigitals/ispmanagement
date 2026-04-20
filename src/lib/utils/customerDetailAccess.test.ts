import { describe, expect, it } from 'vitest';

import {
  getVisibleCustomerDetailTabs,
  normalizeCustomerDetailTab,
  readCustomerDetailTabFromUrlValue,
  getCustomerDetailAutoLoadKey,
  shouldAutoLoadCustomerDetailTab,
  type CustomerDetailAccessState,
} from './customerDetailAccess';

const technicianAccess: CustomerDetailAccessState = {
  canReadCustomerLocations: true,
  canReadBilling: false,
  canReadPppoe: true,
  canReadAudit: false,
};

describe('customer detail access helpers', () => {
  it('hides subscriptions and billing tabs when billing access is missing', () => {
    expect(getVisibleCustomerDetailTabs(technicianAccess)).toEqual([
      'overview',
      'locations',
      'pppoe',
    ]);
  });

  it('keeps subscriptions and billing visible for billing-capable roles', () => {
    expect(
      getVisibleCustomerDetailTabs({
        canReadCustomerLocations: true,
        canReadBilling: true,
        canReadPppoe: true,
        canReadAudit: true,
      }),
    ).toEqual(['overview', 'locations', 'subscriptions', 'billing', 'pppoe', 'timeline']);
  });

  it('normalizes forbidden tabs back to overview', () => {
    expect(normalizeCustomerDetailTab('subscriptions', technicianAccess)).toBe('overview');
    expect(normalizeCustomerDetailTab('billing', technicianAccess)).toBe('overview');
    expect(normalizeCustomerDetailTab('pppoe', technicianAccess)).toBe('pppoe');
  });

  it('does not override local state when the url has no tab value', () => {
    expect(readCustomerDetailTabFromUrlValue('', technicianAccess)).toBeNull();
    expect(readCustomerDetailTabFromUrlValue(null, technicianAccess)).toBeNull();
    expect(readCustomerDetailTabFromUrlValue('locations', technicianAccess)).toBe('locations');
  });

  it('marks pppoe as an auto-loading tab when access is available', () => {
    expect(shouldAutoLoadCustomerDetailTab('pppoe', technicianAccess)).toBe(true);
    expect(shouldAutoLoadCustomerDetailTab('timeline', technicianAccess)).toBe(false);
  });

  it('keeps forbidden tabs from auto-loading', () => {
    expect(shouldAutoLoadCustomerDetailTab('billing', technicianAccess)).toBe(false);
    expect(shouldAutoLoadCustomerDetailTab('subscriptions', technicianAccess)).toBe(false);
  });

  it('builds a stable auto-load key from tab and customer only when allowed', () => {
    expect(getCustomerDetailAutoLoadKey('pppoe', 'customer-1', technicianAccess)).toBe(
      'pppoe:customer-1',
    );
    expect(getCustomerDetailAutoLoadKey('billing', 'customer-1', technicianAccess)).toBeNull();
    expect(getCustomerDetailAutoLoadKey('pppoe', '', technicianAccess)).toBeNull();
  });
});
