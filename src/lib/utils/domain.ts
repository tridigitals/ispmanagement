/**
 * Domain utilities.
 * Browser-side host hints are intentionally weak: tenant authority should come from the backend.
 */

import { secureGetItem } from './tauri-store';

function normalizeHostname(hostname: string): string {
  return String(hostname || '').trim().toLowerCase().replace(/^https?:\/\//, '').replace(/\/+$/, '').replace(/\.+$/, '');
}

function getConfiguredMainDomains(): string[] {
  const list = [
    normalizeHostname(String(import.meta.env.VITE_MAIN_DOMAIN || '')),
    normalizeHostname(String(import.meta.env.VITE_APP_MAIN_DOMAIN || '')),
    ...(import.meta.env.VITE_ALLOWED_HOSTS || '').split(',').map(normalizeHostname).filter(Boolean),
  ].filter(Boolean);

  return Array.from(new Set(list));
}

export function isPlatformDomain(hostname: string): boolean {
  const host = normalizeHostname(hostname);
  if (!host) return false;
  return getConfiguredMainDomains().includes(host);
}

function isLocalHostname(hostname: string): boolean {
  const host = normalizeHostname(hostname);
  return host.includes('localhost') || host.includes('127.0.0.1') || host.includes('tauri');
}

function getStoredTenantSlug(): string | null {
  try {
    const rawUser = secureGetItem('auth_user') || 'null';
    const rawTenant = secureGetItem('auth_tenant') || 'null';
    const rawActiveSlug = secureGetItem('active_tenant_slug') || '';
    const authUser = JSON.parse(rawUser);
    const authTenant = JSON.parse(rawTenant);
    const tenantSlug = String(authUser?.tenant_slug || authTenant?.slug || rawActiveSlug || '').trim();
    return tenantSlug || null;
  } catch {
    return null;
  }
}

function readCachedDomainMapping(hostname: string): string | null {
  if (typeof localStorage === 'undefined' || typeof localStorage.getItem !== 'function') {
    return null;
  }

  try {
    const cache = JSON.parse(localStorage.getItem('tenant_domain_map') || '{}');
    const slug = cache[hostname];
    return typeof slug === 'string' && slug.trim() ? slug.trim() : null;
  } catch (e) {
    console.error('Failed to parse domain map cache', e);
    return null;
  }
}

export function getSlugFromDomain(hostname: string): string | null {
  const host = normalizeHostname(hostname);
  if (!host) return null;

  if (typeof window === 'undefined') {
    return null;
  }

  if (isLocalHostname(host) || isPlatformDomain(host)) {
    return getStoredTenantSlug();
  }

  return readCachedDomainMapping(host);
}

export function cacheDomainMapping(domain: string, slug: string) {
  if (typeof localStorage === 'undefined' || typeof localStorage.setItem !== 'function') return;

  const host = normalizeHostname(domain);
  const cleanSlug = String(slug || '').trim();
  if (!host || !cleanSlug) return;

  try {
    const cache = JSON.parse(localStorage.getItem('tenant_domain_map') || '{}');
    cache[host] = cleanSlug;
    localStorage.setItem('tenant_domain_map', JSON.stringify(cache));
  } catch (e) {
    console.error('Failed to update domain map cache', e);
  }
}
