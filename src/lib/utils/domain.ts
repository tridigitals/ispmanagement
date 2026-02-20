/**
 * Domain Utility
 * resolving hostnames to tenant slugs.
 */

function getConfiguredMainDomains(): string[] {
  const list = [
    String(import.meta.env.VITE_MAIN_DOMAIN || '').trim().toLowerCase(),
    String(import.meta.env.VITE_APP_MAIN_DOMAIN || '').trim().toLowerCase(),
    // Fallback default for this deployment
    'billing.tridigitals.com',
  ].filter(Boolean);
  return Array.from(new Set(list));
}

export function isPlatformDomain(hostname: string): boolean {
  const host = String(hostname || '').trim().toLowerCase();
  if (!host) return false;
  return getConfiguredMainDomains().includes(host);
}

// In a real implementation, this might fetch from an API or use a build-time map.
// For a client-side SPA/Tauri app, this is tricky because we can't easily do blocking HTTP calls in `reroute`.
// We might need to rely on a global config object injected at build time or assume a convention.

export function getSlugFromDomain(hostname: string): string | null {
  // Avoid touching Web APIs during SSR/prerender/build-time.
  if (typeof window === 'undefined') {
    return domainMapFromFallback(hostname);
  }

  // Development overrides
  if (
    hostname.includes('localhost') ||
    hostname.includes('127.0.0.1') ||
    hostname.includes('tauri')
  ) {
    return null; // Don't rewrite localhost
  }

  // Main SaaS/platform domain:
  // keep public URL clean, but allow internal tenant reroute when user already logged in.
  if (isPlatformDomain(hostname)) {
    try {
      const rawUser =
        localStorage.getItem('auth_user') || sessionStorage.getItem('auth_user') || 'null';
      const rawTenant =
        localStorage.getItem('auth_tenant') || sessionStorage.getItem('auth_tenant') || 'null';
      const rawActiveSlug =
        localStorage.getItem('active_tenant_slug') ||
        sessionStorage.getItem('active_tenant_slug') ||
        '';
      const authUser = JSON.parse(rawUser);
      const authTenant = JSON.parse(rawTenant);
      const tenantSlug = String(
        authUser?.tenant_slug || authTenant?.slug || rawActiveSlug || '',
      ).trim();
      return tenantSlug || null;
    } catch {
      return null;
    }
  }

  // TEST ONLY: Force localhost -> tridigitals
  // if (hostname.includes('localhost')) {
  //     return 'tridigitals';
  // }

  // Check LocalStorage cache (Dynamic mappings from first-visit check)
  if (typeof localStorage !== 'undefined' && typeof localStorage.getItem === 'function') {
    try {
      const cache = JSON.parse(localStorage.getItem('tenant_domain_map') || '{}');
      if (cache[hostname]) {
        return cache[hostname];
      }
    } catch (e) {
      console.error('Failed to parse domain map cache', e);
    }
  }

  // Example: dashboard.tridigitals.com -> dashboard
  // Last-resort convention fallback for subdomains only when cache doesn't exist.
  const parts = hostname.split('.');
  if (parts.length >= 3) {
    const candidate = String(parts[0] || '').toLowerCase();
    const reserved = new Set(['www', 'api', 'app', 'cdn', 'mail', 'smtp', 'imap']);
    if (candidate && !reserved.has(candidate)) {
      return candidate;
    }
  }

  return domainMapFromFallback(hostname);
}

function domainMapFromFallback(hostname: string): string | null {
  // Example Hardcoded Mapping (Keep as fallback or remove if desired)
  const domainMap: Record<string, string> = {
    'dashboard.tridigitals.com': 'tridigitals',
    'saas.tridigitals.com': 'tridigitals',
    'my.custom-domain.com': 'another-tenant',
  };

  return domainMap[hostname] || null;
}

/**
 * Helper to cache a new domain mapping
 */
export function cacheDomainMapping(domain: string, slug: string) {
  if (typeof localStorage === 'undefined' || typeof localStorage.setItem !== 'function') return;

  try {
    const cache = JSON.parse(localStorage.getItem('tenant_domain_map') || '{}');
    cache[domain] = slug;
    localStorage.setItem('tenant_domain_map', JSON.stringify(cache));
  } catch (e) {
    console.error('Failed to update domain map cache', e);
  }
}
