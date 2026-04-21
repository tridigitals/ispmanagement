import { describe, expect, it } from 'vitest';

import { getAdminCustomerNavigation } from './adminCustomerNavigation';

describe('admin customer navigation helpers', () => {
  it('builds tenant-prefixed customer routes on platform hosts', () => {
    expect(
      getAdminCustomerNavigation({
        hostname: 'localhost',
        tenantSlug: 'demo',
      }),
    ).toEqual({
      tenantPrefix: '/demo',
      customersPath: '/demo/admin/customers',
    });
  });

  it('keeps customer routes clean on mapped custom domains', () => {
    expect(
      getAdminCustomerNavigation({
        hostname: 'my.custom-domain.com',
        tenantSlug: 'another-tenant',
      }),
    ).toEqual({
      tenantPrefix: '',
      customersPath: '/admin/customers',
    });
  });
});
