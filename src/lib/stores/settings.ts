import { writable, derived } from 'svelte/store';
import { api } from '$lib/api/client';
import { getCurrentWindow } from '@tauri-apps/api/window';

// Tipe data setting
export interface AppSettings {
    app_name: string;
    app_version: string;
    app_description: string;
    organization_name: string;
    support_email: string;
    maintenance_mode: boolean;
    default_locale: string;
    currency_symbol: string;
    [key: string]: any; // Allow indexing
}

// Default values jika database kosong
const defaults: AppSettings = {
    app_name: 'SaaS Boilerplate',
    app_version: '1.0.0',
    app_description: 'The ultimate foundation for your next big idea.',
    organization_name: 'My Company Inc.',
    support_email: 'support@example.com',
    maintenance_mode: false,
    default_locale: 'en-US',
    currency_symbol: '$'
};

function createSettingsStore() {
    const { subscribe, set, update } = writable<AppSettings>(defaults);

    return {
        subscribe,
        // Load initial settings from backend
        init: async () => {
            try {
                const data = await api.settings.getAll();
                const settingsMap: any = { ...defaults };
                
                data.forEach(item => {
                    // Convert boolean strings to actual booleans
                    if (item.value === 'true') settingsMap[item.key] = true;
                    else if (item.value === 'false') settingsMap[item.key] = false;
                    else settingsMap[item.key] = item.value;
                });
                
                set(settingsMap);
                updateWindowTitle(settingsMap.app_name);
            } catch (err) {
                console.error("Failed to load settings:", err);
            }
        },
        // Update a specific setting immediately (Optimistic UI)
        updateSetting: (key: string, value: any) => {
            update(s => {
                const newState = { ...s, [key]: value };
                if (key === 'app_name') updateWindowTitle(value);
                return newState;
            });
        }
    };
}

async function updateWindowTitle(title: string) {
    if (typeof window === 'undefined') return;
    try {
        await getCurrentWindow().setTitle(title);
    } catch (e) {
        console.warn("Failed to set window title:", e);
    }
}

export const appSettings = createSettingsStore();

// Derived stores helper
export const isMaintenanceMode = derived(appSettings, $s => $s.maintenance_mode);
export const appName = derived(appSettings, $s => $s.app_name);