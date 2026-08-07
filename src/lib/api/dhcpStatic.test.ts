import { beforeEach, describe, expect, it, vi } from 'vitest';

const getTokenOrThrow = vi.fn();
const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow,
  safeInvoke,
}));

describe('dhcpStatic api wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getTokenOrThrow.mockReturnValue('token-123');
    safeInvoke.mockResolvedValue({});
  });

  it('passes work order scope when applying a service', async () => {
    const { dhcpStatic } = await import('./dhcpStatic');

    await dhcpStatic.services.apply('service-1', { work_order_id: 'wo-1' });

    expect(safeInvoke).toHaveBeenCalledWith('apply_dhcp_static_service', {
      token: 'token-123',
      id: 'service-1',
      work_order_id: 'wo-1',
    });
  });

  it('reconciles selected router with both command argument aliases', async () => {
    const { dhcpStatic } = await import('./dhcpStatic');

    await dhcpStatic.services.reconcileRouter('router-1');

    expect(safeInvoke).toHaveBeenCalledWith('reconcile_dhcp_static_router', {
      token: 'token-123',
      routerId: 'router-1',
      router_id: 'router-1',
    });
  });
});
