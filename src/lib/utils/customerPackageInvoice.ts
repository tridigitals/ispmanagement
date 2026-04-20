type InvoiceLike = {
  external_id?: string | null;
};

type SubscriptionOptionLike = {
  id: string;
  customerId: string;
  label: string;
  status: string;
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
