/**
 * Domain Utility
 * resolving hostnames to tenant slugs.
 */

// In a real implementation, this might fetch from an API or use a build-time map.
// For a client-side SPA/Tauri app, this is tricky because we can't easily do blocking HTTP calls in `reroute`.
// We might need to rely on a global config object injected at build time or assume a convention.

export function getSlugFromDomain(hostname: string): string | null {
    // Development overrides
    if (hostname.includes('localhost') || hostname.includes('127.0.0.1') || hostname.includes('tauri')) {
        return null; // Don't rewrite localhost
    }

    // TEST ONLY: Force localhost -> tridigitals
    // if (hostname.includes('localhost')) {
    //     return 'tridigitals';
    // }

    // Example: dashboard.tridigitals.com -> tridigitals
    // Checks for subdomains
    const parts = hostname.split('.');
    if (parts.length >= 3) {
        // Assuming structure: [slug].[app].[com]
        // return parts[0]; 
    }

    // Example Hardcoded Mapping
    const domainMap: Record<string, string> = {
        'dashboard.tridigitals.com': 'tridigitals',
        'saas.tridigitals.com': 'tridigitals',
        'my.custom-domain.com': 'another-tenant'
    };

    return domainMap[hostname] || null;
}
