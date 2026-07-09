import type { Reroute } from '@sveltejs/kit';
import { isPlatformDomain } from '$lib/utils/domain';

// SvelteKit may import this optional export in some builds.
// Keeping it defined avoids Rollup "missing export" warnings.
export const transport = undefined;

/**
 * Reroute hook kept only for legacy platform base-path compatibility.
 *
 * Clean tenant app routes now exist physically at root under `src/routes/(app)`.
 * Slug-prefixed app routes are intentionally no longer rewritten internally.
 */
export const reroute: Reroute = ({ url }) => {
  const onPlatformDomain = isPlatformDomain(url.hostname);

  // Example: /isp-management/dashboard -> /dashboard
  if (onPlatformDomain) {
    if (url.pathname === '/isp-management' || url.pathname.startsWith('/isp-management/')) {
      return url.pathname.replace(/^\/isp-management/, '') || '/';
    }
  }

  // Returning undefined means "use the default routing"
  return undefined;
};
