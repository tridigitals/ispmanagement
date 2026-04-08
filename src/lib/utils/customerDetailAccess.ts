export const customerDetailTabs = [
  'overview',
  'locations',
  'subscriptions',
  'billing',
  'timeline',
  'pppoe',
] as const;

export type CustomerDetailTab = (typeof customerDetailTabs)[number];

export type CustomerDetailAccessState = {
  canReadCustomerLocations: boolean;
  canReadBilling: boolean;
  canReadPppoe: boolean;
  canReadAudit: boolean;
};

export function getVisibleCustomerDetailTabs(
  access: CustomerDetailAccessState,
): CustomerDetailTab[] {
  const tabs: CustomerDetailTab[] = ['overview'];

  if (access.canReadCustomerLocations) {
    tabs.push('locations');
  }

  if (access.canReadBilling) {
    tabs.push('subscriptions', 'billing');
  }

  if (access.canReadPppoe) {
    tabs.push('pppoe');
  }

  if (access.canReadAudit) {
    tabs.push('timeline');
  }

  return tabs;
}

export function normalizeCustomerDetailTab(
  tab: string | null | undefined,
  access: CustomerDetailAccessState,
): CustomerDetailTab {
  const normalized = String(tab || '').toLowerCase() as CustomerDetailTab;
  return getVisibleCustomerDetailTabs(access).includes(normalized) ? normalized : 'overview';
}

export function readCustomerDetailTabFromUrlValue(
  tab: string | null | undefined,
  access: CustomerDetailAccessState,
): CustomerDetailTab | null {
  if (tab == null) return null;
  const trimmed = String(tab).trim();
  if (!trimmed) return null;
  return normalizeCustomerDetailTab(trimmed, access);
}
