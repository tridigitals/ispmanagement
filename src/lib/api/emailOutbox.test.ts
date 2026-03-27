import { beforeEach, describe, expect, it, vi } from 'vitest';

const getTokenOrThrow = vi.fn();
const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow,
  safeInvoke,
}));

describe('emailOutbox api wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getTokenOrThrow.mockReturnValue('token-123');
  });

  it('passes list filters through to safeInvoke', async () => {
    safeInvoke.mockResolvedValue({ data: [], total: 0, page: 2, per_page: 50 });
    const { emailOutbox } = await import('./emailOutbox');

    await emailOutbox.list({
      scope: 'all',
      page: 2,
      perPage: 50,
      status: 'queued',
      search: 'ops@example.com',
    });

    expect(safeInvoke).toHaveBeenCalledWith('list_email_outbox', {
      token: 'token-123',
      scope: 'all',
      page: 2,
      per_page: 50,
      status: 'queued',
      search: 'ops@example.com',
    });
  });

  it('returns detail contract with retry visibility fields', async () => {
    safeInvoke.mockResolvedValue({
      id: 'outbox-1',
      tenant_id: 'tenant-1',
      to_email: 'ops@example.com',
      subject: 'Subject',
      body: 'Body',
      body_html: null,
      status: 'queued',
      attempts: 2,
      max_attempts: 5,
      scheduled_at: '2026-03-18T08:00:00Z',
      last_error: 'smtp timeout',
      sent_at: null,
      created_at: '2026-03-18T07:55:00Z',
      updated_at: '2026-03-18T08:00:00Z',
      last_attempted_at: '2026-03-18T08:00:00Z',
      next_retry_at: '2026-03-18T08:05:00Z',
      retryable: true,
      delivery_status_summary: 'Retry scheduled after failed attempt 2 of 5',
    });

    const { emailOutbox } = await import('./emailOutbox');
    const item = await emailOutbox.get('outbox-1');

    expect(safeInvoke).toHaveBeenCalledWith('get_email_outbox', {
      token: 'token-123',
      id: 'outbox-1',
    });
    expect(item.retryable).toBe(true);
    expect(item.last_attempted_at).toBe('2026-03-18T08:00:00Z');
    expect(item.next_retry_at).toBe('2026-03-18T08:05:00Z');
    expect(item.delivery_status_summary).toBe('Retry scheduled after failed attempt 2 of 5');
  });
});
