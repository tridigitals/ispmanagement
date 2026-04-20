import { describe, expect, it } from 'vitest';

import {
  buildCustomerPackageInvoiceRelationFromSubscription,
  findCustomerPackageInvoiceRelation,
  getCustomerPackageSubscriptionId,
} from './customerPackageInvoice';

describe('customer package invoice helpers', () => {
  it('extracts subscription id from pkgsub external ids', () => {
    expect(getCustomerPackageSubscriptionId('pkgsub:sub-1')).toBe('sub-1');
    expect(getCustomerPackageSubscriptionId('pkgsub:sub-1:monthly')).toBe('sub-1');
  });

  it('returns null for unrelated external ids', () => {
    expect(getCustomerPackageSubscriptionId(null)).toBeNull();
    expect(getCustomerPackageSubscriptionId('plan:pro')).toBeNull();
  });

  it('matches invoice relations against loaded subscription options', () => {
    expect(
      findCustomerPackageInvoiceRelation({ external_id: 'pkgsub:sub-99:monthly' }, [
        {
          id: 'sub-99',
          customerId: 'cust-1',
          label: 'Acme - 20 Mbps',
          status: 'active',
        },
      ]),
    ).toEqual({
      subscriptionId: 'sub-99',
      customerId: 'cust-1',
      label: 'Acme - 20 Mbps',
      status: 'active',
    });
  });

  it('builds invoice relation directly from a single subscription payload', () => {
    expect(
      buildCustomerPackageInvoiceRelationFromSubscription(
        { external_id: 'pkgsub:sub-99:monthly' },
        {
          id: 'sub-99',
          customer_id: 'cust-1',
          status: 'active',
          package_name: 'Acme - 20 Mbps',
        },
      ),
    ).toEqual({
      subscriptionId: 'sub-99',
      customerId: 'cust-1',
      label: 'Acme - 20 Mbps',
      status: 'active',
    });
  });

  it('returns null when single subscription does not match invoice external id', () => {
    expect(
      buildCustomerPackageInvoiceRelationFromSubscription(
        { external_id: 'pkgsub:sub-99:monthly' },
        {
          id: 'sub-100',
          customer_id: 'cust-1',
          status: 'active',
          package_name: 'Acme - 20 Mbps',
        },
      ),
    ).toBeNull();
  });
});
