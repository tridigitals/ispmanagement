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

function splitPathAndSuffix(path: string): { pathname: string; suffix: string } {
  const raw = String(path || '/').trim() || '/';
  const match = raw.match(/^([^?#]*)(.*)$/);
  return {
    pathname: match?.[1] || '/',
    suffix: match?.[2] || '',
  };
}

function normalizePath(path: string): string {
  const { pathname, suffix } = splitPathAndSuffix(path);
  const normalized = `/${pathname.split('/').filter(Boolean).join('/')}`;
  return `${normalized === '/' ? '/' : normalized}${suffix}`;
}

export function canonicalTenantPath(path: string): string {
  return normalizePath(path);
}

export function legacyTenantPath(slug: string, path: string): string {
  const cleanSlug = normalize(slug).replace(/^\/+|\/+$/g, '');
  const cleanPath = canonicalTenantPath(path);
  return cleanSlug ? `/${cleanSlug}${cleanPath === '/' ? '' : cleanPath}` : cleanPath;
}

export function resolveTenantContext(input: TenantContextInput): TenantContext {
  const hostname = normalize(input.hostname).toLowerCase();
  const domainSlug = getSlugFromDomain(hostname);
  const effectiveTenantSlug =
    normalize(input.tenantSlug) || normalize(input.userTenantSlug) || normalize(input.routeTenantSlug);
  const onPlatformDomain = isPlatformDomain(hostname);
  const isCustomDomain =
    !!domainSlug && !!effectiveTenantSlug && domainSlug.toLowerCase() === effectiveTenantSlug.toLowerCase();
  const tenantPrefix = '';

  return {
    domainSlug,
    effectiveTenantSlug,
    isCustomDomain,
    onPlatformDomain,
    tenantPrefix,
  };
}
