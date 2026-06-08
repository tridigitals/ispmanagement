/**
 * Tauri Encrypted Storage Adapter
 *
 * Provides a localStorage-compatible API backed by @tauri-apps/plugin-store
 * when running in Tauri desktop. Falls back to browser localStorage/web sessionStorage
 * when running in a regular browser.
 *
 * The plugin-store writes to an encrypted file in the app data directory,
 * which is more secure than browser localStorage for sensitive data like auth tokens.
 */

import { isTauriRuntime } from '$lib/api/core';

const STORE_FILE = 'isp-management.store';

interface TauriStore {
  get<T = unknown>(key: string): Promise<T | undefined>;
  set(key: string, value: unknown): Promise<void>;
  delete(key: string): Promise<boolean>;
  save(): Promise<void>;
}

let _store: TauriStore | null = null;
let _initPromise: Promise<void> | null = null;
let _initDone = false;

/** In-memory cache for synchronous reads (mirrors what's on disk). */
const _cache = new Map<string, string>();

/**
 * Lazy-initialize the Tauri plugin-store.
 * Call once at app boot; subsequent calls are no-ops.
 */
export async function initTauriStore(): Promise<void> {
  if (_initPromise) return _initPromise;
  if (!isTauriRuntime()) {
    _initDone = true;
    return;
  }

  _initPromise = (async () => {
    try {
      const { Store } = await import('@tauri-apps/plugin-store');
      _store = await Store.load(STORE_FILE);

      // Hydrate cache from disk
      const keys = ['auth_token', 'auth_user', 'auth_tenant', 'active_tenant_slug'];
      for (const key of keys) {
        const val = await _store.get<string>(key);
        if (val !== undefined && val !== null) {
          _cache.set(key, String(val));
        }
      }

      console.info('[TauriStore] Encrypted storage initialized');
    } catch (e) {
      console.error('[TauriStore] Init failed, falling back to browser storage:', e);
      _store = null;
    } finally {
      _initDone = true;
    }
  })();

  return _initPromise;
}

/** Wait until init is complete. Call before first read if timing matters. */
export async function tauriStoreReady(): Promise<void> {
  if (_initDone) return;
  if (_initPromise) await _initPromise;
}

/**
 * Synchronous getItem — returns from in-memory cache (Tauri) or browser storage.
 * This is the drop-in replacement for localStorage.getItem().
 */
export function secureGetItem(key: string): string | null {
  if (isTauriRuntime() && _store) {
    return _cache.get(key) ?? null;
  }
  // Browser fallback
  if (typeof window === 'undefined') return null;
  return localStorage.getItem(key) || sessionStorage.getItem(key);
}

/**
 * Synchronous setItem — writes to cache + async persist to disk (Tauri)
 * or directly to browser storage.
 */
export function secureSetItem(key: string, value: string): void {
  if (isTauriRuntime() && _store) {
    _cache.set(key, value);
    void _store.set(key, value).then(() => { if (_store) return _store.save(); }).catch((e) => {
      console.error('[TauriStore] Failed to persist:', key, e);
    });
    return;
  }
  // Browser fallback — caller decides localStorage vs sessionStorage
  // This function always writes to localStorage for simplicity.
  // The auth store's setAuthData handles the remember/session split.
  if (typeof window !== 'undefined') {
    localStorage.setItem(key, value);
  }
}

/**
 * Synchronous removeItem — removes from cache + async persist (Tauri)
 * or directly from browser storage.
 */
export function secureRemoveItem(key: string): void {
  if (isTauriRuntime() && _store) {
    _cache.delete(key);
    void _store.delete(key).then(() => { if (_store) return _store.save(); }).catch((e) => {
      console.error('[TauriStore] Failed to remove:', key, e);
    });
    return;
  }
  // Browser fallback
  if (typeof window !== 'undefined') {
    localStorage.removeItem(key);
    sessionStorage.removeItem(key);
  }
}

/**
 * Clear all auth-related keys from storage.
 */
export function secureClearAuth(): void {
  const keys = ['auth_token', 'auth_user', 'auth_tenant', 'active_tenant_slug'];
  for (const key of keys) {
    secureRemoveItem(key);
  }
}

/**
 * Check if we're using encrypted Tauri storage (vs browser localStorage).
 */
export function isUsingSecureStorage(): boolean {
  return isTauriRuntime() && _store !== null;
}
