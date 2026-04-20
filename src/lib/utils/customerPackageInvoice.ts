type InvoiceLike = {
  external_id?: string | null;
};

type SubscriptionOptionLike = {
  id: string;
  customerId: string;
  label: string;
  status: string;
};

type SubscriptionLike = {
  id: string;
  customer_id: string;
  status: string;
  package_name?: string | null;
};

export function getCustomerPackageSubscriptionId(externalId?: string | null): string | null {
  const value = String(externalId || '').trim();
  if (!value.startsWith('pkgsub:')) return null;
  const parts = value.split(':');
  const subscriptionId = String(parts[1] || '').trim();
  return subscriptionId || null;
}

export function findCustomerPackageInvoiceRelation(
  invoice: InvoiceLike,
  subscriptions: SubscriptionOptionLike[],
) {
  const subscriptionId = getCustomerPackageSubscriptionId(invoice.external_id);
  if (!subscriptionId) return null;

  const matched = subscriptions.find((item) => item.id === subscriptionId);
  if (!matched) return null;

  return {
    subscriptionId,
    customerId: matched.customerId,
    label: matched.label,
    status: matched.status,
  };
}

export function buildCustomerPackageInvoiceRelationFromSubscription(
  invoice: InvoiceLike,
  subscription: SubscriptionLike | null | undefined,
) {
  const subscriptionId = getCustomerPackageSubscriptionId(invoice.external_id);
  if (!subscriptionId || !subscription || subscription.id !== subscriptionId) return null;

  return {
    subscriptionId,
    customerId: subscription.customer_id,
    label: subscription.package_name || subscription.id,
    status: subscription.status,
  };
}
