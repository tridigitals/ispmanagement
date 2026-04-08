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

  const announcementMatch = appRelativePath.match(/^\/announcements\/([^/?#]+)/);
  if (announcementMatch?.[1]) {
    return getAnnouncementDetailPath(announcementMatch[1], opts);
  }

  if (
    appRelativePath.startsWith('/admin') ||
    appRelativePath.startsWith('/support') ||
    appRelativePath.startsWith('/dashboard') ||
    appRelativePath.startsWith('/announcements') ||
    appRelativePath.startsWith('/profile') ||
    appRelativePath.startsWith('/notifications')
  ) {
    return `${tenantPrefix}${appRelativePath}`;
  }

  return actionUrl;
}
