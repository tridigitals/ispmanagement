import { canonicalTenantPath } from './tenantRouting';

type AdminBillingNavigationInput = {
  hostname: string;
  userTenantSlug?: string | null;
  tenantSlug?: string | null;
  routeTenantSlug?: string | null;
};

export function getAdminBillingNavigation(input: AdminBillingNavigationInput) {
  void input;

  return {
    tenantPrefix: '',
    billingPath: canonicalTenantPath('/admin/invoices'),
    collectionsPath: canonicalTenantPath('/admin/invoices/collection'),
    billingPlanSettingsPath: canonicalTenantPath('/admin/settings#billing_plan'),
    subscriptionPath: canonicalTenantPath('/admin/subscription'),
  };
}
