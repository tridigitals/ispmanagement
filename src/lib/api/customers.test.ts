import { beforeEach, describe, expect, it, vi } from 'vitest';

const getTokenOrThrow = vi.fn();
const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow,
  safeInvoke,
}));

describe('customers api wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getTokenOrThrow.mockReturnValue('token-123');
  });

  it('passes lifecycle observability filter through to safeInvoke', async () => {
    safeInvoke.mockResolvedValue({
      generated_at: '2026-03-18T11:00:00Z',
      lifecycle_funnel: [],
      work_order_funnel: [],
      aging_buckets: [],
    });

    const { customers } = await import('./customers');
    await customers.observability.lifecycle('cust-1');

    expect(safeInvoke).toHaveBeenCalledWith('get_customer_lifecycle_observability', {
      token: 'token-123',
      customerId: 'cust-1',
      customer_id: 'cust-1',
    });
  });

  it('accepts installation_done_awaiting_payment in the lifecycle contract', async () => {
    safeInvoke.mockResolvedValue({
      generated_at: '2026-03-18T11:00:00Z',
      lifecycle_funnel: [
        { stage: 'pending_installation', count: 3 },
        { stage: 'installation_done_awaiting_payment', count: 2 },
      ],
      work_order_funnel: [{ stage: 'completed', count: 2 }],
      aging_buckets: [{ bucket: '>7d', count: 1 }],
    });

    const { customers } = await import('./customers');
    const metrics = await customers.observability.lifecycle();

    expect(metrics.lifecycle_funnel[1]?.stage).toBe('installation_done_awaiting_payment');
    expect(metrics.aging_buckets[0]?.bucket).toBe('>7d');
  });
});
