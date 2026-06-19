import { canonicalTenantPath } from './tenantRouting';

export function getAnnouncementDetailPath(
  id: string,
  opts: { tenantPrefix?: string; internal: boolean },
): string {
  const base = opts.internal ? '/admin/announcements' : '/announcements';
  return canonicalTenantPath(`${base}/${id}`);
}

export function resolveAnnouncementActionUrl(
  actionUrl: string,
  opts: { tenantPrefix?: string; internal: boolean },
): string {
  if (!actionUrl) return actionUrl;

  const tenantPrefix = opts.tenantPrefix || '';
  const appRelativePath =
    tenantPrefix && actionUrl.startsWith(`${tenantPrefix}/`)
      ? actionUrl.slice(tenantPrefix.length)
      : actionUrl;

  const normalizedPath =
    appRelativePath === '/dashboard/packages' ? '/dashboard/services' : appRelativePath;

  // Invoice notifications use /pay/{id} — admin should go to invoice detail
  if (opts.internal && normalizedPath.startsWith('/pay/')) {
    const invoiceId = normalizedPath.replace('/pay/', '');
    return canonicalTenantPath(`/admin/invoices/${invoiceId}`);
  }

  const announcementMatch = normalizedPath.match(/^\/announcements\/([^/?#]+)/);
  if (announcementMatch?.[1]) {
    return getAnnouncementDetailPath(announcementMatch[1], opts);
  }

  if (
    normalizedPath.startsWith('/admin') ||
    normalizedPath.startsWith('/support') ||
    normalizedPath.startsWith('/dashboard') ||
    normalizedPath.startsWith('/announcements') ||
    normalizedPath.startsWith('/profile') ||
    normalizedPath.startsWith('/notifications')
  ) {
    return canonicalTenantPath(normalizedPath);
  }

  return actionUrl;
}
