import { writable } from 'svelte/store';
import type { BankAccount } from '$lib/api/client';

export const superadminPlatformSettingsCache = writable<{
  settingsMap: Record<string, string>;
  bankAccounts: BankAccount[];
  fetchedAt: number;
}>({
  settingsMap: {},
  bankAccounts: [],
  fetchedAt: 0,
});
