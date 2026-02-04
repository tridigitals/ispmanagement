import { writable } from 'svelte/store';
import { api } from '$lib/api/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Image } from '@tauri-apps/api/image';

const LOGO_STORAGE_KEY = 'app_logo_cached';

// Get stored logo from localStorage
function getStoredLogo(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem(LOGO_STORAGE_KEY);
}

// Save logo to localStorage
function saveLogoToStorage(logo: string | null) {
  if (typeof window === 'undefined') return;
  if (logo) {
    localStorage.setItem(LOGO_STORAGE_KEY, logo);
  }
  // Note: We don't remove on null to keep last tenant's logo after logout
}

function createLogoStore() {
  // Initialize with stored logo (persisted from last session)
  const { subscribe, set } = writable<string | null>(getStoredLogo());

  const updateWindowIcon = async (base64String: string) => {
    if (typeof window === 'undefined') return;

    // Skip if not running in Tauri
    // @ts-ignore
    if (!window.__TAURI_INTERNALS__) return;

    try {
      // Strip prefix if present (data:image/png;base64,...)
      const base64 = base64String.split(',')[1] || base64String;

      // Decode base64 to bytes
      const binaryString = atob(base64);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }

      // Create Tauri Image and set icon
      const image = await Image.fromBytes(bytes);

      await getCurrentWindow().setIcon(image);
    } catch (err) {
      console.warn('Failed to set window icon:', err);
    }
  };

  // Apply stored logo on startup if exists
  const storedLogo = getStoredLogo();
  if (storedLogo) {
    updateWindowIcon(storedLogo);
  }

  return {
    subscribe,
    set: (value: string | null) => {
      set(value);
      saveLogoToStorage(value);
      if (value) updateWindowIcon(value);
    },
    init: async () => {
      // Load from localStorage on init
      const stored = getStoredLogo();
      if (stored) {
        set(stored);
        updateWindowIcon(stored);
      }
    },
    refresh: async (token?: string) => {
      try {
        const logo = await api.settings.getLogo(token);
        if (logo) {
          set(logo);
          saveLogoToStorage(logo);
          updateWindowIcon(logo);
        } else {
        }
      } catch (err) {
        console.error('[LogoStore] Failed to load logo:', err);
      }
    },
  };
}

export const appLogo = createLogoStore();
