import { writable, derived, get } from 'svelte/store';
import type { AuthSettings } from '$lib/api/types';
import { settings as settingsApi } from '$lib/api/settings';
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
      const canReadSettings = get(can)('read', 'settings');
      const [authSettingsResult, publicSettingsResult, tenantSettingsResult, appVersionResult] =
        await Promise.allSettled([
          settingsApi.getAuthSettings(),
          settingsApi.getPublicSettings(),
          canReadSettings ? settingsApi.getAll() : Promise.resolve([]),
          settingsApi.getAppVersion(),
        ]);

      const authSettings =
        authSettingsResult.status === 'fulfilled' ? authSettingsResult.value : undefined;

      const publicSettings =
        publicSettingsResult.status === 'fulfilled' && publicSettingsResult.value
          ? publicSettingsResult.value
          : {};

      const tenantSettings: Record<string, any> = {};
      if (tenantSettingsResult.status === 'fulfilled') {
        tenantSettingsResult.value.forEach((item) => {
          if (item.value === 'true') tenantSettings[item.key] = true;
          else if (item.value === 'false') tenantSettings[item.key] = false;
          else tenantSettings[item.key] = item.value;
        });
      }

      const appVersion =
        appVersionResult.status === 'fulfilled' ? appVersionResult.value : defaults.app_version;

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
