import { describe, expect, it } from 'vitest';

import { getAdminBillingNavigation } from './adminBillingNavigation';

describe('admin billing navigation helpers', () => {
  it('builds tenant-prefixed billing routes on non-platform hosts', () => {
    expect(
      getAdminBillingNavigation({
        hostname: 'localhost',
        tenantSlug: 'demo',
      }),
    ).toEqual({
      tenantPrefix: '/demo',
      billingPath: '/demo/admin/invoices',
      collectionsPath: '/demo/admin/invoices/collection',
      billingPlanSettingsPath: '/demo/admin/settings?tab=billing_plan',
      subscriptionPath: '/demo/admin/subscription',
    });
  });

  it('keeps billing routes clean on mapped custom domains', () => {
    expect(
      getAdminBillingNavigation({
        hostname: 'my.custom-domain.com',
        tenantSlug: 'another-tenant',
      }),
    ).toEqual({
      tenantPrefix: '',
      billingPath: '/admin/invoices',
      collectionsPath: '/admin/invoices/collection',
      billingPlanSettingsPath: '/admin/settings?tab=billing_plan',
      subscriptionPath: '/admin/subscription',
    });
  });
});
