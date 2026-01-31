import { writable } from "svelte/store";
import type { Setting } from "$lib/api/client";

export type AdminSettingsCacheEntry = {
    settings: Setting[];
    tenantInfo: any;
    customDomainAccess: boolean;
    logoBase64: string | null;
    fetchedAt: number;
};

// Keyed by tenant id/slug (best-effort).
export const adminSettingsCache = writable<Record<string, AdminSettingsCacheEntry>>(
    {},
);

