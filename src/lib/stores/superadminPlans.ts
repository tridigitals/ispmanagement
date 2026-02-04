import { writable } from 'svelte/store';

export type SuperadminPlan = {
  id: string;
  name: string;
  slug: string;
  description: string | null;
  price_monthly: number;
  price_yearly: number;
  is_active: boolean;
  is_default: boolean;
  sort_order: number;
};

export const superadminPlansCache = writable<{
  plans: SuperadminPlan[];
  fetchedAt: number;
}>({
  plans: [],
  fetchedAt: 0,
});
