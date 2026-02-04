import { writable, get } from 'svelte/store';
import { api } from '$lib/api/client';

type TenantsCache = {
  tenants: any[];
  fetchedAt: number | null;
};

export const superadminTenantsCache = writable<TenantsCache>({
  tenants: [],
  fetchedAt: null,
});

const DEFAULT_MAX_AGE_MS = 60_000; // 1 minute

export async function getTenantsCached(options?: {
  force?: boolean;
  maxAgeMs?: number;
}): Promise<any[]> {
  const force = options?.force ?? false;
  const maxAgeMs = options?.maxAgeMs ?? DEFAULT_MAX_AGE_MS;

  const cache = get(superadminTenantsCache);
  const now = Date.now();
  const fresh = cache.fetchedAt !== null && now - cache.fetchedAt < maxAgeMs;

  if (!force && fresh && cache.tenants.length) return cache.tenants;

  const res = await api.superadmin.listTenants();
  const tenants = res?.data || [];
  superadminTenantsCache.set({ tenants, fetchedAt: now });
  return tenants;
}
