import type { Reroute } from '@sveltejs/kit';
import { getSlugFromDomain, isPlatformDomain } from '$lib/utils/domain';

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

const APP_ROOT_SEGMENTS = [
  'admin',
  'dashboard',
  'profile',
  'support',
  'notifications',
  'announcements',
  'storage',
];

/**
 * Reroute hook to handle custom domains and rewrite paths to /[tenant]/...
 * This allows a domain like `dashboard.tridigitals.com/settings` to be
 * internally routed to `/tridigitals/(app)/admin/settings` while keeping the URL clean.
 */
export const reroute: Reroute = ({ url }) => {
  const onPlatformDomain = isPlatformDomain(url.hostname);

  // Block legacy tenant-slug URLs on main platform domain.
  // Example: /isp-management/dashboard -> /dashboard
  if (onPlatformDomain) {
    if (url.pathname === '/isp-management' || url.pathname.startsWith('/isp-management/')) {
      const cleanPath = url.pathname.replace(/^\/isp-management/, '') || '/';
      return cleanPath;
    }

    // Canonicalize any legacy slug-prefixed app path:
    // /:slug/admin/... -> /admin/...
    // /:slug/dashboard/... -> /dashboard/...
    const m = url.pathname.match(/^\/([^/]+)\/(admin|dashboard|profile|support|notifications|announcements|storage)(\/.*)?$/);
    if (m) {
      const firstSegment = m[1];
      if (APP_ROOT_SEGMENTS.includes(firstSegment)) {
        return undefined;
      }
      const appRoot = m[2];
      const tail = m[3] || '';
      return `/${appRoot}${tail}`;
    }
  }

  // On main platform domain, only rewrite tenant-aware APP paths.
  // Never rewrite root/public auth pages (/, /login, /register, etc).
  if (onPlatformDomain) {
    const APP_ROOTS = APP_ROOT_SEGMENTS.map((s) => `/${s}`);
    const isTenantAppPath = APP_ROOTS.some(
      (p) => url.pathname === p || url.pathname.startsWith(p + '/'),
    );
    if (!isTenantAppPath) {
      return undefined;
    }
  }

  const slug = getSlugFromDomain(url.hostname);

  if (slug) {
    // Skip rerouting for public paths that exist at root level
    if (PUBLIC_PATHS.some((p) => url.pathname === p || url.pathname.startsWith(p + '/'))) {
      return undefined;
    }

    // Rewrite path to include slug if not already present
    // e.g. /dashboard -> /tridigitals/dashboard
    if (url.pathname.startsWith(`/${slug}`)) {
      return url.pathname;
    }
    return `/${slug}${url.pathname}`;
  }

  // Returning undefined means "use the default routing"
  return undefined;
};
