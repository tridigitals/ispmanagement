import { writable, derived } from 'svelte/store';
import { api, type AuthSettings } from '$lib/api/client';
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
    auth?: AuthSettings; // Dynamic auth settings
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
    currency_symbol: '$',
    auth: undefined
};

function createSettingsStore() {
    const { subscribe, set, update } = writable<AppSettings>(defaults);

    return {
        subscribe,
        // Load initial settings from backend
        init: async () => {
            try {
                // Fetch public auth settings
                const authSettings = await api.settings.getAuthSettings();
                
                // Fetch generic app settings (might fail if not logged in, which is fine)
                let generalSettings: any = {};
                try {
                   const data = await api.settings.getAll();
                   data.forEach(item => {
                       if (item.value === 'true') generalSettings[item.key] = true;
                       else if (item.value === 'false') generalSettings[item.key] = false;
                       else generalSettings[item.key] = item.value;
                   });
                } catch (e) {
                    // Ignore error if not logged in (api.settings.getAll requires token)
                    console.debug("Could not load admin settings (likely not logged in)");
                }

                const finalSettings = { 
                    ...defaults, 
                    ...generalSettings,
                    auth: authSettings 
                };
                
                set(finalSettings);
                updateWindowTitle(finalSettings.app_name);
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