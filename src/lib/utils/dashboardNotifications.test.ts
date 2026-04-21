import { describe, expect, it } from 'vitest';

import type { Notification } from '$lib/api/types';

import {
  getDashboardRecentNotifications,
  getVisiblePortalNotifications,
} from './dashboardNotifications';

function notification(overrides: Partial<Notification>): Notification {
  return {
    id: overrides.id || 'n-1',
    user_id: overrides.user_id || 'u-1',
    tenant_id: overrides.tenant_id ?? 't-1',
    title: overrides.title || 'Title',
    message: overrides.message || 'Message',
    notification_type: overrides.notification_type || 'info',
    category: overrides.category || 'system',
    action_url: overrides.action_url ?? null,
    is_read: overrides.is_read ?? false,
    created_at: overrides.created_at || '2026-04-21T00:00:00Z',
  };
}

describe('getDashboardRecentNotifications', () => {
  it('hides legacy invoice reminder notifications for portal users', () => {
    const got = getDashboardRecentNotifications(
      [
        notification({
          id: 'legacy-reminder',
          category: 'billing',
          title: 'Invoice overdue by 3 day(s)',
          action_url: '/dashboard/invoices',
        }),
        notification({
          id: 'valid-payment-link',
          category: 'billing',
          title: 'Invoice created',
          action_url: '/pay/invoice-123',
        }),
        notification({
          id: 'other-category',
          category: 'announcement',
          action_url: '/announcements/ann-1',
        }),
      ],
      false,
      6,
    );

    expect(got.map((item) => item.id)).toEqual(['valid-payment-link', 'other-category']);
  });

  it('hides pay links that do not belong to the portal user invoice set', () => {
    const got = getDashboardRecentNotifications(
      [
        notification({
          id: 'foreign-invoice',
          category: 'billing',
          title: 'Invoice created',
          action_url: '/pay/foreign-invoice-id',
        }),
        notification({
          id: 'owned-invoice',
          category: 'billing',
          title: 'Invoice created',
          action_url: '/pay/owned-invoice-id',
        }),
      ],
      false,
      6,
      ['owned-invoice-id'],
    );

    expect(got.map((item) => item.id)).toEqual(['owned-invoice']);
  });

  it('keeps legacy invoice reminders visible for internal users', () => {
    const rows = [
      notification({
        id: 'legacy-reminder',
        category: 'billing',
        title: 'Invoice overdue by 3 day(s)',
        action_url: '/dashboard/invoices',
      }),
    ];

    expect(getDashboardRecentNotifications(rows, true, 6).map((item) => item.id)).toEqual([
      'legacy-reminder',
    ]);
  });

  it('limits after filtering so portal users still get up to the requested number of valid items', () => {
    const rows = [
      notification({
        id: 'legacy-reminder-1',
        category: 'billing',
        title: 'Invoice overdue by 1 day(s)',
        action_url: '/dashboard/invoices',
      }),
      notification({
        id: 'keep-1',
        category: 'support',
      }),
      notification({
        id: 'legacy-reminder-2',
        category: 'billing',
        title: 'Invoice due today',
        action_url: '/dashboard/invoices',
      }),
      notification({
        id: 'keep-2',
        category: 'billing',
        title: 'Invoice created',
        action_url: '/pay/invoice-456',
      }),
    ];

    expect(getDashboardRecentNotifications(rows, false, 2).map((item) => item.id)).toEqual([
      'keep-1',
      'keep-2',
    ]);
  });
});

describe('getVisiblePortalNotifications', () => {
  it('keeps all notifications for internal users', () => {
    const rows = [
      notification({
        id: 'legacy-reminder',
        category: 'billing',
        title: 'Invoice overdue by 3 day(s)',
        action_url: '/dashboard/invoices',
      }),
      notification({
        id: 'foreign-invoice',
        category: 'billing',
        title: 'Invoice created',
        action_url: '/pay/foreign-invoice-id',
      }),
    ];

    expect(getVisiblePortalNotifications(rows, true, ['owned-invoice-id']).map((item) => item.id)).toEqual([
      'legacy-reminder',
      'foreign-invoice',
    ]);
  });
});
