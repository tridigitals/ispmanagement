type ServiceCatalogUserLike = {
  role?: string | null;
  tenant_role?: string | null;
  is_super_admin?: boolean | null;
};

const SERVICE_CATALOG_ALLOWED_ROLES = new Set(['owner', 'admin', 'sales', 'backoffice']);

export function canAccessServiceCatalog(
  user: ServiceCatalogUserLike | null | undefined,
  canReadIspPackages: boolean,
  canManageIspPackages: boolean,
): boolean {
  if (!canReadIspPackages && !canManageIspPackages) return false;
  if (!user) return false;
  if (user.is_super_admin) return true;

  const role = String(user.tenant_role || user.role || '')
    .trim()
    .toLowerCase();

  return SERVICE_CATALOG_ALLOWED_ROLES.has(role);
}
