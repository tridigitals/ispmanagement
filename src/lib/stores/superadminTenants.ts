import { writable } from 'svelte/store';

export type SuperadminTenant = {
  id: string;
  name: string;
  slug: string;
  custom_domain?: string | null;
  is_active: boolean;
  created_at: string;
};

export const superadminTenantsCache = writable<{
  tenants: SuperadminTenant[];
  fetchedAt: number;
}>({
  tenants: [],
  fetchedAt: 0,
});
