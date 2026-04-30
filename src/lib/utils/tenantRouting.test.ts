import { describe, expect, it } from 'vitest';

import { canonicalTenantPath, legacyTenantPath, resolveTenantContext } from './tenantRouting';

describe('tenant routing helpers', () => {
  it('builds clean canonical tenant app paths', () => {
    expect(canonicalTenantPath('admin/settings')).toBe('/admin/settings');
    expect(canonicalTenantPath('/dashboard')).toBe('/dashboard');
  });

  it('preserves query strings', () => {
    expect(canonicalTenantPath('/profile?tab=security')).toBe('/profile?tab=security');
  });

  it('normalizes duplicate slashes', () => {
    expect(canonicalTenantPath('//admin//settings')).toBe('/admin/settings');
  });

  it('builds explicit legacy paths only when requested', () => {
    expect(legacyTenantPath('demo', '/admin/settings')).toBe('/demo/admin/settings');
  });

  it('keeps tenant prefix empty for canonical navigation even when a slug is known', () => {
    expect(
      resolveTenantContext({
        hostname: 'localhost',
        tenantSlug: 'demo',
      }).tenantPrefix,
    ).toBe('');
  });
});
