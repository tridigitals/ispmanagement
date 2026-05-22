export type LandingUserLike = {
  role?: string | null;
  is_super_admin?: boolean | null;
  permissions?: string[] | null;
};

const INTERNAL_PERMISSION_PREFIXES = [
  'admin:',
  'team:',
  'roles:',
  'settings:',
  'customers:',
  'customer_locations:',
  'billing:',
  'work_orders:',
  'pppoe:',
  'network_',
  'router_inventory:',
  'ppp_profiles:',
  'ip_pools:',
  'isp_packages:',
  'audit_logs:',
  'email_outbox:',
  'storage_console:',
  'backups:',
];

const PORTAL_ONLY_PERMISSIONS = new Set(['customers:read_own']);

export function hasInternalAppAccess(user: LandingUserLike | null | undefined): boolean {
  if (!user) return false;
  if (user.is_super_admin) return true;

  const role = String(user.role || '').toLowerCase();
  if (role === 'owner' || role === 'admin') return true;

  const permissions = Array.isArray(user.permissions) ? user.permissions : [];
  if (permissions.includes('*') || permissions.includes('admin:access')) return true;

  return permissions.some((permission) => {
    if (PORTAL_ONLY_PERMISSIONS.has(permission)) return false;
    return INTERNAL_PERMISSION_PREFIXES.some((prefix) => permission.startsWith(prefix));
  });
}

export function getDefaultTenantLandingPath(
  user: LandingUserLike | null | undefined,
  _tenantPrefix: string,
): string {
  return hasInternalAppAccess(user) ? '/admin' : '/dashboard';
}

export function canAccessCustomerDashboard(user: LandingUserLike | null | undefined): boolean {
  return !hasInternalAppAccess(user);
}
