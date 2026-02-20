import { getSlugFromDomain, isPlatformDomain } from '$lib/utils/domain';

export const APP_ROOT_SEGMENTS = [
  'admin',
  'dashboard',
  'profile',
  'support',
  'notifications',
  'announcements',
  'storage',
] as const;

type TenantContextInput = {
  hostname: string;
  userTenantSlug?: string | null;
  tenantSlug?: string | null;
  routeTenantSlug?: string | null;
};

type TenantContext = {
  domainSlug: string | null;
  effectiveTenantSlug: string;
  isCustomDomain: boolean;
  onPlatformDomain: boolean;
  tenantPrefix: string;
};

function normalize(v?: string | null): string {
  return String(v || '').trim();
}

export function resolveTenantContext(input: TenantContextInput): TenantContext {
  const hostname = normalize(input.hostname).toLowerCase();
  const domainSlug = getSlugFromDomain(hostname);
  const effectiveTenantSlug =
    normalize(input.tenantSlug) || normalize(input.userTenantSlug) || normalize(input.routeTenantSlug);
  const onPlatformDomain = isPlatformDomain(hostname);
  const isCustomDomain =
    !!domainSlug && !!effectiveTenantSlug && domainSlug.toLowerCase() === effectiveTenantSlug.toLowerCase();
  const tenantPrefix =
    effectiveTenantSlug && !isCustomDomain && !onPlatformDomain ? `/${effectiveTenantSlug}` : '';

  return {
    domainSlug,
    effectiveTenantSlug,
    isCustomDomain,
    onPlatformDomain,
    tenantPrefix,
  };
}
