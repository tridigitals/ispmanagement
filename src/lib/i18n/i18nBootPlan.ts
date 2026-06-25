export const I18N_ALL_NAMESPACES = [
  'auth',
  'common',
  'components',
  'pages',
  'install',
  'payment',
  'sidebar',
  'topbar',
  'support',
  'announcements',
  'notifications_page',
  'notifications',
  'utils',
  'superadmin',
  'dashboard',
  'profile',
  'admin',
  'network',
  'mixradius',
] as const;

export type I18nNamespace = (typeof I18N_ALL_NAMESPACES)[number];
export type AppLocale = 'en' | 'id';

const BASE_NAMESPACES: I18nNamespace[] = [
  'common',
  'auth',
  'pages',
  'sidebar',
  'topbar',
  'install',
  'payment',
  'components',
  'utils',
  'network',
  'mixradius',
];

export function normalizeAppLocale(input: string | null | undefined): AppLocale {
  const normalized = String(input || '')
    .trim()
    .replaceAll('_', '-')
    .toLowerCase();

  if (normalized.startsWith('id')) return 'id';
  return 'en';
}

export function resolveBootNamespaces(pathname: string): I18nNamespace[] {
  const path = String(pathname || '').toLowerCase();
  const namespaces = new Set<I18nNamespace>(BASE_NAMESPACES);

  if (path.includes('/superadmin')) namespaces.add('superadmin');
  if (path.includes('/admin')) namespaces.add('admin');
  if (path.includes('/dashboard')) {
    namespaces.add('dashboard');
    namespaces.add('profile');
  }
  if (path.includes('/profile')) namespaces.add('profile');
  if (path.includes('/support')) namespaces.add('support');
  if (path.includes('/announcement')) namespaces.add('announcements');
  if (path.includes('/notification')) {
    namespaces.add('notifications');
    namespaces.add('notifications_page');
  }

  return [...namespaces];
}
