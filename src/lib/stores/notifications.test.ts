import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

const apiNotifications = {
  list: vi.fn(),
  getUnreadCount: vi.fn(),
  markAsRead: vi.fn(),
  markAllAsRead: vi.fn(),
  delete: vi.fn(),
  getPreferences: vi.fn(),
  updatePreference: vi.fn(),
  subscribePush: vi.fn(),
  unsubscribePush: vi.fn(),
  sendTest: vi.fn(),
};

vi.mock('$lib/api/client', () => ({
  notifications: apiNotifications,
}));

vi.mock('svelte-sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock('@tauri-apps/plugin-notification', () => ({
  sendNotification: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: vi.fn(async () => false),
}));

describe('notifications store', () => {
  beforeEach(async () => {
    vi.resetAllMocks();

    const module = await import('./notifications');
    module.notifications.set([]);
    module.unreadCount.set(0);
  });

  it('uses API unread count when available', async () => {
    const module = await import('./notifications');
    module.notifications.set([
      { id: 'n-1', is_read: false },
      { id: 'n-2', is_read: true },
    ] as any);
    apiNotifications.getUnreadCount.mockResolvedValue({ count: 7 });

    await module.refreshUnreadCount(true);

    expect(get(module.unreadCount)).toBe(7);
  });

  it('falls back to in-memory unread notifications when API unread count fails', async () => {
    const module = await import('./notifications');
    module.notifications.set([
      { id: 'n-1', is_read: false },
      { id: 'n-2', is_read: true },
      { id: 'n-3', is_read: false },
    ] as any);
    apiNotifications.getUnreadCount.mockRejectedValue(new Error('network down'));

    await module.refreshUnreadCount(true);

    expect(get(module.unreadCount)).toBe(2);
  });
});
