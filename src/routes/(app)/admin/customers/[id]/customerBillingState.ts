import type { Invoice } from '$lib/api/client';

export type CustomerBillingFilter = 'all' | 'unpaid' | 'paid' | 'overdue';

type BillingDependencies = {
  invoices: Invoice[];
  subscriptionById: Map<string, unknown>;
  getSubscriptionIdFromInvoice: (invoice: Invoice) => string | null;
  now?: number;
};

function hasLinkedSubscription(
  invoice: Invoice,
  subscriptionById: Map<string, unknown>,
  getSubscriptionIdFromInvoice: (invoice: Invoice) => string | null,
) {
  const subscriptionId = getSubscriptionIdFromInvoice(invoice);
  return !!subscriptionId && subscriptionById.has(subscriptionId);
}

function isInvoiceUnpaid(invoice: Invoice) {
  return invoice.status === 'pending' || invoice.status === 'verification_pending';
}

function isInvoiceOverdue(invoice: Invoice, now: number) {
  return invoice.status !== 'paid' && new Date(invoice.due_date).getTime() < now;
}

export function filterCustomerBillingRows(args: BillingDependencies & { filter: CustomerBillingFilter }) {
  const { invoices, subscriptionById, getSubscriptionIdFromInvoice, filter, now = Date.now() } = args;

  return invoices
    .filter((invoice) => hasLinkedSubscription(invoice, subscriptionById, getSubscriptionIdFromInvoice))
    .filter((invoice) => {
      if (filter === 'all') return true;
      if (filter === 'unpaid') return isInvoiceUnpaid(invoice);
      if (filter === 'paid') return invoice.status === 'paid';
      return isInvoiceOverdue(invoice, now);
    })
    .sort(
      (a, b) =>
        new Date(b.created_at || b.due_date).getTime() - new Date(a.created_at || a.due_date).getTime(),
    );
}

export function buildCustomerBillingStats(args: BillingDependencies) {
  const { invoices, subscriptionById, getSubscriptionIdFromInvoice, now = Date.now() } = args;
  const eligible = invoices.filter((invoice) =>
    hasLinkedSubscription(invoice, subscriptionById, getSubscriptionIdFromInvoice),
  );

  return {
    total: eligible.length,
    unpaid: eligible.filter(isInvoiceUnpaid).length,
    paid: eligible.filter((invoice) => invoice.status === 'paid').length,
    overdue: eligible.filter((invoice) => isInvoiceOverdue(invoice, now)).length,
  };
}
