import { describe, expect, it } from 'vitest';

import { getAdminCustomerNavigation } from './adminCustomerNavigation';

describe('admin customer navigation helpers', () => {
  it('builds clean customer routes by default', () => {
    expect(
      getAdminCustomerNavigation({
        hostname: 'localhost',
        tenantSlug: 'demo',
      }),
    ).toEqual({
      tenantPrefix: '',
      customersPath: '/admin/customers',
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
