import { beforeEach, describe, expect, it, vi } from 'vitest';

const getTokenOrThrow = vi.fn();
const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow,
  safeInvoke,
}));

describe('payment api wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getTokenOrThrow.mockReturnValue('token-123');
  });

  it('passes customer package invoice status and date filters', async () => {
    safeInvoke.mockResolvedValue({
      data: [],
      total: 0,
      page: 1,
      per_page: 25,
    });

    const { payment } = await import('./payment');
    await payment.listCustomerPackageInvoices({
      sort_by: 'due_date',
      sort_dir: 'asc',
      status: 'verification_pending',
      created_from: '2026-08-01T00:00:00.000Z',
      created_to: '2026-08-05T23:59:59.999Z',
      page: 1,
      per_page: 25,
    });

    expect(safeInvoke).toHaveBeenCalledWith('list_customer_package_invoices', {
      token: 'token-123',
      sort_by: 'due_date',
      sort_dir: 'asc',
      status: 'verification_pending',
      created_from: '2026-08-01T00:00:00.000Z',
      created_to: '2026-08-05T23:59:59.999Z',
      page: 1,
      per_page: 25,
    });
  });
});
