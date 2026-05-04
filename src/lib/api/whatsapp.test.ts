import { beforeEach, describe, expect, it, vi } from 'vitest';

const getTokenOrThrow = vi.fn();
const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow,
  safeInvoke,
}));

describe('whatsapp api wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getTokenOrThrow.mockReturnValue('token-123');
  });

  it('sends test WhatsApp messages through safeInvoke', async () => {
    safeInvoke.mockResolvedValue({ ok: true, provider: 'fonnte', message_id: 'msg-1' });
    const { whatsapp } = await import('./whatsapp');

    await whatsapp.sendTest({
      phone: '+628123456789',
      message: 'Connectivity test',
      eventCode: 'customer_invoice_due',
    });

    expect(safeInvoke).toHaveBeenCalledWith('send_test_whatsapp', {
      token: 'token-123',
      phone: '+628123456789',
      message: 'Connectivity test',
      eventCode: 'customer_invoice_due',
    });
  });

  it('lists WhatsApp events with the auth token', async () => {
    safeInvoke.mockResolvedValue([
      { code: 'customer_invoice_due', label: 'Invoice due', scope: 'tenant' },
    ]);
    const { whatsapp } = await import('./whatsapp');

    await whatsapp.listEvents();

    expect(safeInvoke).toHaveBeenCalledWith('list_whatsapp_events', {
      token: 'token-123',
    });
  });

  it('checks gateway readiness with the auth token', async () => {
    safeInvoke.mockResolvedValue({ ready: true, provider: 'fonnte', reason: null });
    const { whatsapp } = await import('./whatsapp');

    await whatsapp.readiness();

    expect(safeInvoke).toHaveBeenCalledWith('get_whatsapp_gateway_readiness', {
      token: 'token-123',
    });
  });

  it('sends customer WhatsApp messages through the gateway', async () => {
    safeInvoke.mockResolvedValue({ ok: true, provider: 'fonnte', status: 200 });
    const { whatsapp } = await import('./whatsapp');

    await whatsapp.sendCustomer({
      customerId: 'cust-1',
      message: 'Halo Andi',
      template: 'custom',
    });

    expect(safeInvoke).toHaveBeenCalledWith('send_customer_whatsapp', {
      token: 'token-123',
      customer_id: 'cust-1',
      customerId: 'cust-1',
      message: 'Halo Andi',
      template: 'custom',
    });
  });
});
