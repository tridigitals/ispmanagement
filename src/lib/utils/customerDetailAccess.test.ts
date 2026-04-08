import { describe, expect, it } from 'vitest';

import {
  getVisibleCustomerDetailTabs,
  normalizeCustomerDetailTab,
  readCustomerDetailTabFromUrlValue,
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
});
