import { describe, expect, it } from 'vitest';
import { isBrightCanvasPath } from './bootTheme';

describe('isBrightCanvasPath', () => {
  it('v2 routes are bright except the NOC wallboard', () => {
    expect(isBrightCanvasPath('/v2/admin')).toBe(true);
    expect(isBrightCanvasPath('/v2/admin/customers')).toBe(true);
    expect(isBrightCanvasPath('/v2/admin/network/noc/wallboard/settings')).toBe(true);
    expect(isBrightCanvasPath('/v2/admin/network/noc/wallboard')).toBe(false);
  });

  it('public auth pages are bright', () => {
    for (const p of ['/login', '/register', '/forgot-password', '/verify-email', '/unauthorized', '/maintenance']) {
      expect(isBrightCanvasPath(p)).toBe(true);
    }
    expect(isBrightCanvasPath('/forgot-password/reset')).toBe(true);
  });

  it('dark-themed surfaces stay dark', () => {
    expect(isBrightCanvasPath('/pay/abc-123')).toBe(false);
    expect(isBrightCanvasPath('/install')).toBe(false);
    expect(isBrightCanvasPath('/superadmin/tenants')).toBe(false);
    expect(isBrightCanvasPath('/admin/customers')).toBe(false);
    expect(isBrightCanvasPath('/')).toBe(false);
  });
});
