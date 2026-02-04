import { writable, derived, get } from 'svelte/store';
import { api, type AuthSettings } from '$lib/api/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { locale } from 'svelte-i18n';
import { can } from '$lib/stores/auth';
import '../i18n'; // Initialize i18n

// Tipe data setting
export interface AppSettings {
  app_name: string;
  app_version: string;
  app_description: string;
  organization_name: string;
  support_email: string;
  maintenance_mode: boolean;
  maintenance_message?: string;
  default_locale: string;
  app_timezone: string;
  // Stable "pricing/base" currency stored in the database (plans, invoices, etc).
  // Tenants choose `currency_code` as their display currency; amounts are converted using FX rates.
  base_currency_code: string;
  // Display currency (tenants may override).
  currency_code: string;
  auth?: AuthSettings; // Dynamic auth settings
  [key: string]: any; // Allow indexing
}

// Default values jika database kosong
const defaults: AppSettings = {
  app_name: import.meta.env.VITE_APP_NAME || 'SaaS Boilerplate',
  app_version: '1.0.0',
  app_description: 'The ultimate foundation for your next big idea.',
  organization_name: 'My Company Inc.',
  support_email: 'support@example.com',
  maintenance_mode: false,
  default_locale: 'en-US',
  app_timezone: 'UTC',
  base_currency_code: 'IDR',
  currency_code: 'IDR',
  auth: undefined,
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
        // console.debug("Could not load public settings", e);
      }

      // Fetch admin/tenant settings (might fail if not logged in, which is fine)
      // If logged in, getAll() will now return tenant-specific settings because backend extracts tenant_id from token
      let tenantSettings: any = {};

      // Only fetch if has permission
      if (get(can)('read', 'settings')) {
        try {
          const data = await api.settings.getAll();
          data.forEach((item) => {
            if (item.value === 'true') tenantSettings[item.key] = true;
            else if (item.value === 'false') tenantSettings[item.key] = false;
            else tenantSettings[item.key] = item.value;
          });
        } catch (e) {
          // Ignore error if not logged in or unauthorized
          // console.debug("Could not load tenant settings (likely not logged in)");
        }
      }

      // Fetch app version from backend (from Cargo.toml)
      let appVersion = defaults.app_version;
      try {
        appVersion = await api.settings.getAppVersion();
      } catch (e) {
        // Use default if fails
      }

      const finalSettings = {
        ...defaults,
        ...publicSettings,
        ...tenantSettings,
        app_version: appVersion,
        auth: authSettings,
        // Ensure global maintenance settings from publicSettings are not overwritten by tenantSettings
        maintenance_mode: (publicSettings as any).maintenance_mode ?? defaults.maintenance_mode,
        maintenance_message: (publicSettings as any).maintenance_message,
        // Ensure base currency stays global/stable (do not allow tenant override).
        base_currency_code:
          (publicSettings as any).base_currency_code ??
          (publicSettings as any).currency_code ??
          defaults.base_currency_code,
      };

      set(finalSettings);

      // Set locale from settings with logging
      if (finalSettings.default_locale) {
        locale.set(finalSettings.default_locale);
      } else {
      }

      updateWindowTitle(finalSettings.app_name);
    } catch (err) {
      console.error('Failed to load settings:', err);
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
    reset: () => {
      set(defaults);
    },
    updateSetting: (key: string, value: any) => {
      update((s) => {
        const newState = { ...s, [key]: value };
        if (key === 'app_name') updateWindowTitle(value);
        return newState;
      });
    },
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
    console.warn('Failed to set window title:', e);
  }
}

export const appSettings = createSettingsStore();

// Derived stores helper
export const isMaintenanceMode = derived(appSettings, ($s) => $s.maintenance_mode);
export const appName = derived(appSettings, ($s) => $s.app_name);
