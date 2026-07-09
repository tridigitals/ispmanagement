import type { Reroute, Handle } from '@sveltejs/kit';
import { isPlatformDomain } from '$lib/utils/domain';

// SvelteKit may import this optional export in some builds.
// Keeping it defined avoids Rollup "missing export" warnings.
export const transport = undefined;

/** Security headers applied to all HTML responses. */
export const handle: Handle = async ({ event, resolve }) => {
  const response = await resolve(event);
  response.headers.set('X-Frame-Options', 'DENY');
  response.headers.set('X-Content-Type-Options', 'nosniff');
  response.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
  response.headers.set('Permissions-Policy', 'camera=(), microphone=(), geolocation=()');
  // CSP: allow self + inline scripts (SvelteKit hydration) + inline styles
  if (!response.headers.has('Content-Security-Policy')) {
    response.headers.set(
      'Content-Security-Policy',
      "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self'; font-src 'self' data:; frame-ancestors 'none'"
    );
  }
  return response;
};

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
