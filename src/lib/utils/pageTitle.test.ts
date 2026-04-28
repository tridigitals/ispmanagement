import { describe, expect, it } from 'vitest';

import { formatDocumentTitle, isTenantScopedPath, resolvePageTitle } from './pageTitle';

describe('page title helpers', () => {
  it('resolves platform page titles', () => {
    expect(resolvePageTitle('/superadmin/settings')).toBe('Platform Settings');
    expect(resolvePageTitle('/superadmin/plans/new')).toBe('New Plan');
    expect(resolvePageTitle('/login')).toBe('Login');
  });

  it('resolves tenant-prefixed routes as tenant app pages', () => {
    expect(resolvePageTitle('/isp-jakarta/admin/settings')).toBe('Settings');
    expect(resolvePageTitle('/isp-jakarta/admin/network/noc/wallboard')).toBe('NOC Wallboard');
    expect(resolvePageTitle('/isp-jakarta/dashboard/services/order/internet')).toBe('Order Service');
  });

  it('detects tenant app paths after optional slug normalization', () => {
    expect(isTenantScopedPath('/admin/settings')).toBe(true);
    expect(isTenantScopedPath('/isp-jakarta/admin/settings')).toBe(true);
    expect(isTenantScopedPath('/superadmin/settings')).toBe(false);
  });

  it('formats page title with app or tenant name suffix', () => {
    expect(formatDocumentTitle('Settings', 'ISP Management')).toBe('Settings | ISP Management');
    expect(formatDocumentTitle('ISP Management', 'ISP Management')).toBe('ISP Management');
    expect(formatDocumentTitle('', '')).toBe('ISP Management');
  });
});
