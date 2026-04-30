import { describe, expect, it } from 'vitest';

import { getAdminBillingNavigation } from './adminBillingNavigation';

describe('admin billing navigation helpers', () => {
  it('builds clean billing routes by default', () => {
    expect(
      getAdminBillingNavigation({
        hostname: 'localhost',
        tenantSlug: 'demo',
      }),
    ).toEqual({
      tenantPrefix: '',
      billingPath: '/admin/invoices',
      collectionsPath: '/admin/invoices/collection',
      billingPlanSettingsPath: '/admin/settings?tab=billing_plan',
      subscriptionPath: '/admin/subscription',
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
