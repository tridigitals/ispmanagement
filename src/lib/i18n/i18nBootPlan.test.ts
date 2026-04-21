import { describe, expect, it } from 'vitest';

import { normalizeAppLocale, resolveBootNamespaces } from './i18nBootPlan';

describe('i18n boot plan', () => {
  it('normalizes browser and app locales to supported base locales', () => {
    expect(normalizeAppLocale('id-ID')).toBe('id');
    expect(normalizeAppLocale('id_ID')).toBe('id');
    expect(normalizeAppLocale('en-US')).toBe('en');
    expect(normalizeAppLocale('fr-FR')).toBe('en');
    expect(normalizeAppLocale('')).toBe('en');
  });

  it('loads base namespaces for public auth routes', () => {
    expect(resolveBootNamespaces('/login')).toEqual([
      'common',
      'auth',
      'pages',
      'sidebar',
      'topbar',
      'install',
      'payment',
      'components',
      'utils',
    ]);
  });

  it('adds admin namespace for admin routes without pulling the full locale upfront', () => {
    expect(resolveBootNamespaces('/acme/admin/network/map')).toContain('admin');
    expect(resolveBootNamespaces('/acme/admin/network/map')).not.toContain('superadmin');
  });

  it('adds superadmin namespace for superadmin routes', () => {
    expect(resolveBootNamespaces('/superadmin/settings')).toContain('superadmin');
  });

  it('adds dashboard and profile buckets for app user routes', () => {
    const namespaces = resolveBootNamespaces('/acme/dashboard');
    expect(namespaces).toContain('dashboard');
    expect(namespaces).toContain('profile');
  });
});
