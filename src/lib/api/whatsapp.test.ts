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
});
