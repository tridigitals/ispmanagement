export function getAnnouncementDetailPath(
  id: string,
  opts: { tenantPrefix: string; internal: boolean },
): string {
  const base = opts.internal ? '/admin/announcements' : '/announcements';
  return `${opts.tenantPrefix}${base}/${id}`;
}

export function resolveAnnouncementActionUrl(
  actionUrl: string,
  opts: { tenantPrefix: string; internal: boolean },
): string {
  if (!actionUrl) return actionUrl;

  const tenantPrefix = opts.tenantPrefix || '';
  const appRelativePath =
    tenantPrefix && actionUrl.startsWith(`${tenantPrefix}/`)
      ? actionUrl.slice(tenantPrefix.length)
      : actionUrl;

  const normalizedPath =
    appRelativePath === '/dashboard/packages' ? '/dashboard/services' : appRelativePath;

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
    return `${tenantPrefix}${normalizedPath}`;
  }

  return actionUrl;
}
