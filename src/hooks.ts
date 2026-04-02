import type { Reroute } from '@sveltejs/kit';
import { getSlugFromDomain, isPlatformDomain } from '$lib/utils/domain';
import { APP_ROOT_SEGMENTS } from '$lib/utils/tenantRouting';

// SvelteKit may import this optional export in some builds.
// Keeping it defined avoids Rollup "missing export" warnings.
export const transport = undefined;

// Paths that should NOT be rerouted to tenant prefix
const PUBLIC_PATHS = [
  '/maintenance',
  '/install',
  '/login',
  '/register',
  '/forgot-password',
  '/reset-password',
  '/superadmin',
];

/**
 * Reroute hook to handle custom domains and rewrite paths to /[tenant]/...
 * This allows a domain like `dashboard.tridigitals.com/settings` to be
 * internally routed to `/tridigitals/(app)/admin/settings` while keeping the URL clean.
 */
export const reroute: Reroute = ({ url }) => {
  const onPlatformDomain = isPlatformDomain(url.hostname);
  let normalizedPath = url.pathname;

  // Block legacy tenant-slug URLs on main platform domain.
  // Example: /isp-management/dashboard -> /dashboard
  if (onPlatformDomain) {
    if (normalizedPath === '/isp-management' || normalizedPath.startsWith('/isp-management/')) {
      normalizedPath = normalizedPath.replace(/^\/isp-management/, '') || '/';
    }

    // Canonicalize any legacy slug-prefixed app path:
    // /:slug/admin/... -> /admin/...
    // /:slug/dashboard/... -> /dashboard/...
    const m = normalizedPath.match(
      /^\/([^/]+)\/(admin|dashboard|profile|support|notifications|announcements|storage)(\/.*)?$/,
    );
    if (m) {
      const firstSegment = m[1];
      if (!(APP_ROOT_SEGMENTS as readonly string[]).includes(firstSegment)) {
        const appRoot = m[2];
        const tail = m[3] || '';
        normalizedPath = `/${appRoot}${tail}`;
      }
    }
  }

  // On main platform domain, only rewrite tenant-aware APP paths.
  // Never rewrite root/public auth pages (/, /login, /register, etc).
  if (onPlatformDomain) {
    const APP_ROOTS = APP_ROOT_SEGMENTS.map((s) => `/${s}`);
    const isTenantAppPath = APP_ROOTS.some(
      (p) => normalizedPath === p || normalizedPath.startsWith(p + '/'),
    );
    if (!isTenantAppPath) {
      return undefined;
    }
  }

  const slug = getSlugFromDomain(url.hostname);

  if (slug) {
    // Skip rerouting for public paths that exist at root level
    if (PUBLIC_PATHS.some((p) => normalizedPath === p || normalizedPath.startsWith(p + '/'))) {
      return undefined;
    }

    // Rewrite path to include slug if not already present
    // e.g. /dashboard -> /tridigitals/dashboard
    if (normalizedPath.startsWith(`/${slug}`)) {
      return normalizedPath;
    }
    return `/${slug}${normalizedPath}`;
  }

  // Returning undefined means "use the default routing"
  return undefined;
};
