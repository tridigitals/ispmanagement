import { describe, expect, it } from 'vitest';

import { getAnnouncementDetailPath, resolveAnnouncementActionUrl } from './announcementRouting';

describe('announcement routing helpers', () => {
  it('builds admin announcement detail paths for internal roles', () => {
    expect(getAnnouncementDetailPath('ann-1', { tenantPrefix: '/demo', internal: true })).toBe(
      '/demo/admin/announcements/ann-1',
    );
  });

  it('builds customer announcement detail paths for portal users', () => {
    expect(getAnnouncementDetailPath('ann-1', { tenantPrefix: '/demo', internal: false })).toBe(
      '/demo/announcements/ann-1',
    );
  });

  it('rewrites generic announcement action urls into admin announcement detail for internal roles', () => {
    expect(
      resolveAnnouncementActionUrl('/announcements/ann-1', {
        tenantPrefix: '/demo',
        internal: true,
      }),
    ).toBe('/demo/admin/announcements/ann-1');
  });

  it('keeps generic announcement action urls in portal space for customer roles', () => {
    expect(
      resolveAnnouncementActionUrl('/announcements/ann-1', {
        tenantPrefix: '/demo',
        internal: false,
      }),
    ).toBe('/demo/announcements/ann-1');
  });

  it('prefixes non-announcement app urls without changing their section', () => {
    expect(
      resolveAnnouncementActionUrl('/notifications', {
        tenantPrefix: '/demo',
        internal: true,
      }),
    ).toBe('/demo/notifications');
  });

  it('rewrites announcement action urls for internal roles on custom domains without tenant prefixes', () => {
    expect(
      resolveAnnouncementActionUrl('/announcements/ann-1', {
        tenantPrefix: '',
        internal: true,
      }),
    ).toBe('/admin/announcements/ann-1');
  });
});
