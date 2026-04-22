import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  ack: vi.fn(),
  success: vi.fn(),
}));

vi.mock('$lib/api/client', () => ({
  api: {
    mikrotik: {
      alerts: {
        ack: mocks.ack,
      },
    },
  },
}));

vi.mock('$lib/stores/toast', () => ({
  toast: {
    success: mocks.success,
  },
}));

import { ackWallboardAlerts } from './wallboardAlertsActions';

describe('ackWallboardAlerts', () => {
  beforeEach(() => {
    mocks.ack.mockReset();
    mocks.success.mockReset();
  });

  it('acknowledges all ids and shows success toast', async () => {
    mocks.ack.mockResolvedValue(undefined);

    await ackWallboardAlerts(['a1', 'a2', 'a3'], 'Acked');

    expect(mocks.ack).toHaveBeenCalledTimes(3);
    expect(mocks.success).toHaveBeenCalledWith('Acked');
  });

  it('throws when one or more requests fail', async () => {
    mocks.ack
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce(new Error('boom'))
      .mockResolvedValueOnce(undefined);

    await expect(ackWallboardAlerts(['a1', 'a2', 'a3'], 'Acked')).rejects.toThrow(
      'Failed to acknowledge 1 alert(s)',
    );
    expect(mocks.success).not.toHaveBeenCalled();
  });
});
