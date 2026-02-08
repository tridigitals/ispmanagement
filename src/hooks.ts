import type { Reroute } from '@sveltejs/kit';
import { getSlugFromDomain } from '$lib/utils/domain';

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
