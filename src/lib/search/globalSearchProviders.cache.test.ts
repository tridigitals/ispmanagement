import { beforeEach, describe, expect, it, vi } from 'vitest';

const listInvoices = vi.fn();
const listRouters = vi.fn();
const listTeam = vi.fn();
const listAllInvoices = vi.fn();

vi.mock('$lib/api/payment', () => ({
  payment: {
    listInvoices,
    listAllInvoices,
  },
}));

vi.mock('$lib/api/mikrotik', () => ({
  mikrotik: {
    routers: {
      list: listRouters,
    },
  },
}));

vi.mock('$lib/api/team', () => ({
  team: {
    list: listTeam,
  },
}));

vi.mock('$lib/api/customers', () => ({
  customers: {
    list: vi.fn(),
  },
}));

vi.mock('$lib/api/support', () => ({
  support: {
    list: vi.fn(),
  },
}));

vi.mock('$lib/api/superadmin', () => ({
  superadmin: {
    listTenants: vi.fn(),
  },
}));

import { getGlobalSearchProviders, resetGlobalSearchProviderCaches } from './globalSearchProviders';
import type { GlobalSearchProviderContext } from './globalSearchModel';

const adminContext: GlobalSearchProviderContext = {
  can: () => true,
  isSuperAdmin: false,
  shellScope: 'admin',
  tenantPrefix: '',
};

describe('globalSearchProviders cache', () => {
  beforeEach(() => {
    resetGlobalSearchProviderCaches();
    listInvoices.mockReset();
    listRouters.mockReset();
    listTeam.mockReset();
    listAllInvoices.mockReset();
  });

  it('reuses cached load-all provider data between searches', async () => {
    listInvoices.mockResolvedValue([
      {
        id: 'inv-1',
        invoice_number: 'INV-1',
        amount: 1,
        status: 'pending',
        description: 'Alpha',
        due_date: '2026-01-01',
        paid_at: null,
        payment_method: null,
      },
    ]);

    const provider = getGlobalSearchProviders().find((item) => item.key === 'invoices');
    if (!provider) throw new Error('invoices provider missing');

    await provider.search('alpha', adminContext);
    await provider.search('inv', adminContext);

    expect(listInvoices).toHaveBeenCalledTimes(1);
  });
});
