import { canonicalTenantPath } from './tenantRouting';

type AdminCustomerNavigationInput = {
  hostname: string;
  userTenantSlug?: string | null;
  tenantSlug?: string | null;
  routeTenantSlug?: string | null;
};

export function getAdminCustomerNavigation(input: AdminCustomerNavigationInput) {
  void input;

  return {
    tenantPrefix: '',
    customersPath: canonicalTenantPath('/admin/customers'),
  };
}
