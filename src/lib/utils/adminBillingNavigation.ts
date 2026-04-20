import { resolveTenantContext } from './tenantRouting';

type AdminBillingNavigationInput = {
  hostname: string;
  userTenantSlug?: string | null;
  tenantSlug?: string | null;
  routeTenantSlug?: string | null;
};

export function getAdminBillingNavigation(input: AdminBillingNavigationInput) {
  const tenantCtx = resolveTenantContext(input);
  const { tenantPrefix } = tenantCtx;

  return {
    tenantPrefix,
    billingPath: `${tenantPrefix}/admin/invoices`,
    collectionsPath: `${tenantPrefix}/admin/invoices/collection`,
    billingPlanSettingsPath: `${tenantPrefix}/admin/settings?tab=billing_plan`,
    subscriptionPath: `${tenantPrefix}/admin/subscription`,
  };
}
