import { describe, expect, it } from 'vitest';

import type { Invoice } from '$lib/api/client';

import {
  buildCustomerBillingStats,
  filterCustomerBillingRows,
  type CustomerBillingFilter,
} from './customerBillingState';

function invoice(
  id: string,
  status: Invoice['status'],
  dueDate: string,
  subscriptionId: string | null,
): Invoice {
  return {
    id,
    invoice_number: id,
    amount: 100_000,
    status,
    description: null,
    due_date: dueDate,
    paid_at: status === 'paid' ? dueDate : null,
    payment_method: status === 'paid' ? 'bank_transfer' : null,
    external_id: subscriptionId ? `pkgsub:${subscriptionId}:invoice` : null,
    created_at: dueDate,
    updated_at: dueDate,
    currency_code: 'IDR',
  } as Invoice;
}

describe('customer billing state', () => {
  const now = new Date('2026-05-22T12:00:00Z').getTime();
  const invoices = [
    invoice('inv-paid', 'paid', '2026-05-20T00:00:00Z', 'sub-1'),
    invoice('inv-pending', 'pending', '2026-05-25T00:00:00Z', 'sub-1'),
    invoice('inv-verif', 'verification_pending', '2026-05-21T00:00:00Z', 'sub-1'),
    invoice('inv-overdue', 'pending', '2026-05-19T00:00:00Z', 'sub-2'),
    invoice('inv-unlinked', 'pending', '2026-05-25T00:00:00Z', null),
  ];
  const subscriptionById = new Map([
    ['sub-1', { id: 'sub-1' }],
    ['sub-2', { id: 'sub-2' }],
  ]);
  const getSubscriptionIdFromInvoice = (row: Invoice) => {
    const externalId = row.external_id || '';
    if (!externalId.startsWith('pkgsub:')) return null;
    return externalId.slice('pkgsub:'.length).split(':')[0] || null;
  };

  it.each([
    ['all', ['inv-pending', 'inv-verif', 'inv-paid', 'inv-overdue']],
    ['unpaid', ['inv-pending', 'inv-verif', 'inv-overdue']],
    ['paid', ['inv-paid']],
    ['overdue', ['inv-verif', 'inv-overdue']],
  ] as Array<[CustomerBillingFilter, string[]]>)(
    'filters %s invoice rows for linked subscriptions only',
    (filter, expectedIds) => {
      const rows = filterCustomerBillingRows({
        invoices,
        subscriptionById,
        getSubscriptionIdFromInvoice,
        filter,
        now,
      });

      expect(rows.map((row) => row.id)).toEqual(expectedIds);
    },
  );

  it('builds stats from the full eligible invoice set', () => {
    expect(
      buildCustomerBillingStats({
        invoices,
        subscriptionById,
        getSubscriptionIdFromInvoice,
        now,
      }),
    ).toEqual({
      total: 4,
      unpaid: 3,
      paid: 1,
      overdue: 2,
    });
  });
});
