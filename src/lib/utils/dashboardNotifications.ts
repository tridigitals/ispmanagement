import type { Notification } from '$lib/api/types';

function isLegacyPortalInvoiceReminder(notification: Notification): boolean {
  if (notification.category !== 'billing') return false;
  if (notification.action_url !== '/dashboard/invoices') return false;

  const title = `${notification.title || ''}`.trim().toLowerCase();
  return title.startsWith('invoice due') || title.startsWith('invoice overdue');
}

function invoiceIdFromActionUrl(actionUrl: string | null): string | null {
  const match = String(actionUrl || '').match(/^\/pay\/([^/?#]+)/);
  return match?.[1] || null;
}

export function getVisiblePortalNotifications(
  notifications: Notification[],
  internal: boolean,
  accessibleInvoiceIds: string[] = [],
): Notification[] {
  const accessibleInvoiceIdSet = new Set(accessibleInvoiceIds);
  return internal
    ? notifications
    : notifications.filter((notification) => {
        if (isLegacyPortalInvoiceReminder(notification)) return false;

        const invoiceId = invoiceIdFromActionUrl(notification.action_url);
        if (invoiceId && accessibleInvoiceIdSet.size > 0 && !accessibleInvoiceIdSet.has(invoiceId)) {
          return false;
        }

        return true;
      });
}

export function getDashboardRecentNotifications(
  notifications: Notification[],
  internal: boolean,
  limit: number = 6,
  accessibleInvoiceIds: string[] = [],
): Notification[] {
  const rows = getVisiblePortalNotifications(notifications, internal, accessibleInvoiceIds);

  return rows.slice(0, limit);
}
