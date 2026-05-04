import { beforeEach, describe, expect, it, vi } from 'vitest';

const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow: () => 'token-123',
  safeInvoke,
}));

describe('message templates api wrapper', () => {
  beforeEach(() => {
    safeInvoke.mockReset();
  });

  it('lists message templates with filters', async () => {
    safeInvoke.mockResolvedValue([
      {
        id: 'tpl-1',
        tenantId: 'tenant-1',
        key: 'outage_customer_notice',
        name: 'Outage - Customer Notice',
        useCase: 'outage',
        target: 'customer',
        triggerMode: 'manual',
        eventKey: 'network.outage_notice',
        channel: 'both',
        locale: 'id-ID',
        status: 'active',
        whatsappBody: 'Halo {{customer.name}}',
        emailSubject: 'Informasi gangguan',
        emailBody: 'Halo {{customer.name}}',
        variables: ['customer.name'],
        version: 1,
        createdAt: '2026-05-04T00:00:00Z',
        updatedAt: '2026-05-04T00:00:00Z',
      },
    ]);
    const { messageTemplates } = await import('./messageTemplates');

    const templates = await messageTemplates.list({ channel: 'whatsapp', status: 'active', target: 'customer' });

    expect(safeInvoke).toHaveBeenCalledWith('list_message_templates', {
      token: 'token-123',
      channel: 'whatsapp',
      status: 'active',
      target: 'customer',
    });
    expect(templates[0]).toMatchObject({
      tenant_id: 'tenant-1',
      use_case: 'outage',
      trigger_mode: 'manual',
      event_key: 'network.outage_notice',
      whatsapp_body: 'Halo {{customer.name}}',
      email_subject: 'Informasi gangguan',
      email_body: 'Halo {{customer.name}}',
    });
  });

  it('creates message templates', async () => {
    safeInvoke.mockResolvedValue({ id: 'tpl-1' });
    const { messageTemplates } = await import('./messageTemplates');

    await messageTemplates.create({
      key: 'greeting',
      name: 'Greeting',
      useCase: 'lifecycle',
      target: 'customer',
      triggerMode: 'manual',
      channel: 'whatsapp',
      status: 'active',
      whatsappBody: 'Halo {{customer.name}}',
    });

    expect(safeInvoke).toHaveBeenCalledWith('create_message_template', {
      token: 'token-123',
      payload: expect.objectContaining({ key: 'greeting' }),
    });
  });

  it('updates and deletes message templates', async () => {
    safeInvoke.mockResolvedValue({ id: 'tpl-1' });
    const { messageTemplates } = await import('./messageTemplates');

    await messageTemplates.update('tpl-1', {
      key: 'greeting',
      name: 'Greeting',
      useCase: 'lifecycle',
      target: 'customer',
      triggerMode: 'manual',
      channel: 'whatsapp',
      status: 'active',
    });
    await messageTemplates.delete('tpl-1');

    expect(safeInvoke).toHaveBeenCalledWith('update_message_template', {
      token: 'token-123',
      id: 'tpl-1',
      payload: expect.objectContaining({ key: 'greeting' }),
    });
    expect(safeInvoke).toHaveBeenCalledWith('delete_message_template', {
      token: 'token-123',
      id: 'tpl-1',
    });
  });

  it('previews message templates', async () => {
    safeInvoke.mockResolvedValue({ whatsappBody: 'Halo Andi' });
    const { messageTemplates } = await import('./messageTemplates');

    await messageTemplates.preview({
      whatsappBody: 'Halo {{customer.name}}',
      context: { customer: { name: 'Andi' } },
    });

    expect(safeInvoke).toHaveBeenCalledWith('preview_message_template', {
      token: 'token-123',
      payload: {
        whatsappBody: 'Halo {{customer.name}}',
        context: { customer: { name: 'Andi' } },
      },
    });
  });
});
