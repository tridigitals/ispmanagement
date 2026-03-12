import { getTokenOrThrow, safeInvoke } from './core';
import type { Notification, NotificationPreference, PaginatedResponse } from './types';

export const notifications = {
  list: (page?: number, perPage?: number): Promise<PaginatedResponse<Notification>> =>
    safeInvoke('list_notifications', { token: getTokenOrThrow(), page, perPage }),

  getUnreadCount: (): Promise<{ count: number }> =>
    safeInvoke('get_unread_count', { token: getTokenOrThrow() }),

  markAsRead: (id: string): Promise<void> =>
    safeInvoke('mark_as_read', { token: getTokenOrThrow(), id }),

  markAllAsRead: (): Promise<void> => safeInvoke('mark_all_as_read', { token: getTokenOrThrow() }),

  delete: (id: string): Promise<void> =>
    safeInvoke('delete_notification', { token: getTokenOrThrow(), id }),

  getPreferences: (): Promise<NotificationPreference[]> =>
    safeInvoke('get_preferences', { token: getTokenOrThrow() }),

  updatePreference: (channel: string, category: string, enabled: boolean): Promise<void> =>
    safeInvoke('update_preference', { token: getTokenOrThrow(), channel, category, enabled }),

  subscribePush: (endpoint: string, p256dh: string, auth: string): Promise<void> =>
    safeInvoke('subscribe_push', { token: getTokenOrThrow(), endpoint, p256dh, auth }),

  unsubscribePush: (endpoint: string): Promise<void> =>
    safeInvoke('unsubscribe_push', { token: getTokenOrThrow(), endpoint }),

  sendTest: (): Promise<void> => safeInvoke('send_test', { token: getTokenOrThrow() }),
};
