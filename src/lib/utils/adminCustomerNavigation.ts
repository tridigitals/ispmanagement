import { resolveTenantContext } from './tenantRouting';

type AdminCustomerNavigationInput = {
  hostname: string;
  userTenantSlug?: string | null;
  tenantSlug?: string | null;
  routeTenantSlug?: string | null;
};

export function getAdminCustomerNavigation(input: AdminCustomerNavigationInput) {
  const tenantCtx = resolveTenantContext(input);
  const { tenantPrefix } = tenantCtx;

  return {
    tenantPrefix,
    customersPath: `${tenantPrefix}/admin/customers`,
  };
}
