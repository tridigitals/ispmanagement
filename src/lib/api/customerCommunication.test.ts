import { beforeEach, describe, expect, it, vi } from 'vitest';

const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow: () => 'token-123',
  safeInvoke,
}));

describe('customer communication api wrapper', () => {
  beforeEach(() => {
    safeInvoke.mockReset();
  });

  it('sends customer email payloads', async () => {
    safeInvoke.mockResolvedValue({ ok: true, queued: true });
    const { customerCommunication } = await import('./customerCommunication');

    await customerCommunication.sendEmail({
      customerId: 'cust-1',
      templateId: 'tpl-1',
      subject: 'Subject',
      body: 'Body',
    });

    expect(safeInvoke).toHaveBeenCalledWith('send_customer_email', {
      token: 'token-123',
      payload: {
        customerId: 'cust-1',
        templateId: 'tpl-1',
        subject: 'Subject',
        body: 'Body',
      },
    });
  });
});
