import { writable, derived } from 'svelte/store';
import { api, type AuthSettings } from '$lib/api/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { locale } from 'svelte-i18n';
import '../i18n'; // Initialize i18n

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

    const loadSettings = async () => {
        try {
            // Fetch public auth settings
            const authSettings = await api.settings.getAuthSettings();
            
            // Fetch public general settings (name, description)
            let publicSettings = {};
            try {
                const ps = await api.settings.getPublicSettings();
                if (ps) publicSettings = ps;
            } catch (e) {
                console.debug("Could not load public settings", e);
            }

            // Fetch admin/tenant settings (might fail if not logged in, which is fine)
            // If logged in, getAll() will now return tenant-specific settings because backend extracts tenant_id from token
            let tenantSettings: any = {};
            try {
               const data = await api.settings.getAll();
               data.forEach(item => {
                   if (item.value === 'true') tenantSettings[item.key] = true;
                   else if (item.value === 'false') tenantSettings[item.key] = false;     
                   else tenantSettings[item.key] = item.value;
               });
            } catch (e) {
                // Ignore error if not logged in
                console.debug("Could not load tenant settings (likely not logged in)");
            }

            const finalSettings = {
                ...defaults,
                ...publicSettings,
                ...tenantSettings,
                auth: authSettings
            };

            set(finalSettings);
            
            // Set locale from settings with logging
            if (finalSettings.default_locale) {
                console.log(`[Settings] Setting locale to: ${finalSettings.default_locale}`);
                locale.set(finalSettings.default_locale);
            } else {
                console.log('[Settings] No default_locale found, using browser default');
            }
            
            updateWindowTitle(finalSettings.app_name);
        } catch (err) {
            console.error("Failed to load settings:", err);
        }
    };

    return {
        subscribe,
        init: async () => {
            await loadSettings();
        },
        refresh: async () => {
            await loadSettings();
        },
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
    
    // Skip if not running in Tauri
    // @ts-ignore
    if (!window.__TAURI_INTERNALS__) {
        document.title = title;
        return;
    }

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