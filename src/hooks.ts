import type { Reroute } from '@sveltejs/kit';
import { getSlugFromDomain } from '$lib/utils/domain';

/**
 * Reroute hook to handle custom domains and rewrite paths to /[tenant]/...
 * This allows a domain like `dashboard.tridigitals.com/settings` to be
 * internally routed to `/tridigitals/(app)/admin/settings` while keeping the URL clean.
 */
export const reroute: Reroute = ({ url }) => {
    const slug = getSlugFromDomain(url.hostname);

    if (slug) {
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
