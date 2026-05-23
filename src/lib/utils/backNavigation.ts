import { canonicalTenantPath } from './tenantRouting';

function normalizeBackTarget(input: string | null | undefined): string | null {
  const value = String(input || '').trim();
  if (!value.startsWith('/') || value.startsWith('//')) return null;
  return canonicalTenantPath(value);
}

export function appendBackParam(targetPath: string, currentUrl: URL): string {
  const target = new URL(canonicalTenantPath(targetPath), currentUrl.origin);
  const backTarget = normalizeBackTarget(`${currentUrl.pathname}${currentUrl.search}${currentUrl.hash}`);

  if (backTarget) target.searchParams.set('back', backTarget);

  return `${target.pathname}${target.search}${target.hash}`;
}

export function resolveBackTarget(pageUrl: URL, fallbackPath: string): string {
  return normalizeBackTarget(pageUrl.searchParams.get('back')) || canonicalTenantPath(fallbackPath);
}
