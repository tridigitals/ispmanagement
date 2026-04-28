const PLATFORM_ROOTS = new Set([
  'login',
  'register',
  'forgot-password',
  'verify-email',
  'install',
  'maintenance',
  'unauthorized',
  'superadmin',
  'pay',
]);

const TENANT_ROOTS = new Set([
  'admin',
  'dashboard',
  'profile',
  'support',
  'notifications',
  'announcements',
  'storage',
]);

const ROUTE_TITLES: Array<[RegExp, string]> = [
  [/^\/$/, 'Home'],
  [/^\/login\/?$/, 'Login'],
  [/^\/register\/?$/, 'Register'],
  [/^\/forgot-password(\/reset)?\/?$/, 'Reset Password'],
  [/^\/verify-email\/?$/, 'Verify Email'],
  [/^\/install\/?$/, 'Install'],
  [/^\/maintenance\/?$/, 'Maintenance'],
  [/^\/unauthorized\/?$/, 'Unauthorized'],
  [/^\/pay\/[^/]+\/?$/, 'Payment'],

  [/^\/superadmin\/?$/, 'Superadmin'],
  [/^\/superadmin\/settings\/?$/, 'Platform Settings'],
  [/^\/superadmin\/users\/?$/, 'Users'],
  [/^\/superadmin\/tenants\/?$/, 'Tenants'],
  [/^\/superadmin\/plans\/new\/?$/, 'New Plan'],
  [/^\/superadmin\/plans\/[^/]+\/?$/, 'Plan Detail'],
  [/^\/superadmin\/plans\/?$/, 'Plans'],
  [/^\/superadmin\/invoices\/[^/]+\/?$/, 'Invoice Detail'],
  [/^\/superadmin\/invoices\/?$/, 'Invoices'],
  [/^\/superadmin\/radius\/?$/, 'Managed Radius'],
  [/^\/superadmin\/storage\/?$/, 'Storage'],
  [/^\/superadmin\/backups\/?$/, 'Backups'],
  [/^\/superadmin\/audit-logs\/?$/, 'Audit Logs'],
  [/^\/superadmin\/system\/?$/, 'System'],

  [/^\/admin\/?$/, 'Admin'],
  [/^\/admin\/settings\/?$/, 'Settings'],
  [/^\/admin\/customers\/[^/]+\/?$/, 'Customer Detail'],
  [/^\/admin\/customers\/?$/, 'Customers'],
  [/^\/admin\/services\/?$/, 'Services'],
  [/^\/admin\/invoices\/collection\/?$/, 'Invoice Collection'],
  [/^\/admin\/invoices\/[^/]+\/?$/, 'Invoice Detail'],
  [/^\/admin\/invoices\/?$/, 'Invoices'],
  [/^\/admin\/subscription\/?$/, 'Subscription'],
  [/^\/admin\/team\/?$/, 'Team'],
  [/^\/admin\/roles\/?$/, 'Roles'],
  [/^\/admin\/support\/[^/]+\/?$/, 'Support Ticket'],
  [/^\/admin\/support\/?$/, 'Support'],
  [/^\/admin\/announcements\/[^/]+\/?$/, 'Announcement Detail'],
  [/^\/admin\/announcements\/?$/, 'Announcements'],
  [/^\/admin\/email-outbox\/?$/, 'Email Outbox'],
  [/^\/admin\/audit-logs\/?$/, 'Audit Logs'],
  [/^\/admin\/storage\/?$/, 'Storage'],
  [/^\/admin\/backups\/?$/, 'Backups'],
  [/^\/admin\/network\/noc\/wallboard\/settings\/?$/, 'Wallboard Settings'],
  [/^\/admin\/network\/noc\/wallboard\/?$/, 'NOC Wallboard'],
  [/^\/admin\/network\/noc\/?$/, 'Network NOC'],
  [/^\/admin\/network\/map\/?$/, 'Network Map'],
  [/^\/admin\/network\/alerts\/?$/, 'Network Alerts'],
  [/^\/admin\/network\/incidents\/?$/, 'Network Incidents'],
  [/^\/admin\/network\/logs\/?$/, 'Network Logs'],
  [/^\/admin\/network\/routers\/[^/]+\/?$/, 'Router Detail'],
  [/^\/admin\/network\/routers\/?$/, 'Routers'],
  [/^\/admin\/network\/ppp-profiles\/?$/, 'PPP Profiles'],
  [/^\/admin\/network\/ip-pools\/?$/, 'IP Pools'],
  [/^\/admin\/network\/pppoe\/import\/?$/, 'Import PPPoE'],
  [/^\/admin\/network\/pppoe\/?$/, 'PPPoE'],
  [/^\/admin\/network\/packages\/?$/, 'Packages'],
  [/^\/admin\/network\/installations\/?$/, 'Installations'],
  [/^\/admin\/network\/import\/mixradius\/?$/, 'MixRadius Import'],
  [/^\/admin\/network\/import\/?$/, 'Network Import'],

  [/^\/dashboard\/locations\/?$/, 'Locations'],
  [/^\/dashboard\/packages\/?$/, 'Packages'],
  [/^\/dashboard\/invoices\/?$/, 'Invoices'],
  [/^\/dashboard\/services\/order\/[^/]+\/?$/, 'Order Service'],
  [/^\/dashboard\/services\/order\/?$/, 'Order Service'],
  [/^\/dashboard\/services\/?$/, 'Services'],
  [/^\/dashboard\/?$/, 'Dashboard'],
  [/^\/profile\/?$/, 'Profile'],
  [/^\/notifications\/?$/, 'Notifications'],
  [/^\/support\/[^/]+\/?$/, 'Support Ticket'],
  [/^\/support\/?$/, 'Support'],
  [/^\/announcements\/[^/]+\/?$/, 'Announcement Detail'],
  [/^\/announcements\/?$/, 'Announcements'],
  [/^\/storage\/?$/, 'Storage'],
];

function normalizePathname(pathname: string): string {
  const rawPath = pathname.split(/[?#]/, 1)[0] || '/';
  const parts = rawPath.split('/').filter(Boolean);

  if (parts.length > 1 && !PLATFORM_ROOTS.has(parts[0]) && TENANT_ROOTS.has(parts[1])) {
    return `/${parts.slice(1).join('/')}`;
  }

  return rawPath === '/' ? '/' : `/${parts.join('/')}`;
}

function titleCaseSegment(segment: string): string {
  return segment
    .split('-')
    .filter(Boolean)
    .map((part) => {
      const upper = part.toUpperCase();
      if (['IP', 'PPP', 'PPPOE', 'NOC', 'VPN'].includes(upper)) return upper;
      return part.charAt(0).toUpperCase() + part.slice(1);
    })
    .join(' ');
}

export function isTenantScopedPath(pathname: string): boolean {
  const normalized = normalizePathname(pathname);
  const first = normalized.split('/').filter(Boolean)[0];
  return !!first && TENANT_ROOTS.has(first);
}

export function resolvePageTitle(pathname: string): string {
  const normalized = normalizePathname(pathname);
  const match = ROUTE_TITLES.find(([pattern]) => pattern.test(normalized));
  if (match) return match[1];

  const parts = normalized.split('/').filter(Boolean);
  const lastReadable = [...parts].reverse().find((part) => !/^[0-9a-f-]{16,}$/i.test(part));
  return lastReadable ? titleCaseSegment(lastReadable) : 'Page';
}

export function formatDocumentTitle(pageTitle: string, appName: string): string {
  const cleanPageTitle = pageTitle.trim();
  const cleanAppName = appName.trim() || 'ISP Management';

  if (!cleanPageTitle || cleanPageTitle === cleanAppName) return cleanAppName;
  return `${cleanPageTitle} | ${cleanAppName}`;
}
