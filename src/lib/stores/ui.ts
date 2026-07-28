import { writable } from 'svelte/store';

/**
 * Global sidebar collapse state (desktop).
 *
 * - `true`  → sidebar rendered icon-only (72px) with hover tooltips.
 * - `false` → sidebar fully expanded with labels.
 *
 * The desktop preference is persisted to localStorage so the user's
 * choice survives reloads. Mobile behaviour is handled separately by
 * the layout (`isMobileOpen`) and always resets this to `false` when
 * the mobile drawer opens, so persistence only applies on desktop.
 */
const STORAGE_KEY = 'ui.sidebar_collapsed';

function loadInitial(): boolean {
  if (typeof window === 'undefined') return false;
  try {
    return window.localStorage.getItem(STORAGE_KEY) === '1';
  } catch {
    return false;
  }
}

export const isSidebarCollapsed = writable(loadInitial());

if (typeof window !== 'undefined') {
  isSidebarCollapsed.subscribe((value) => {
    try {
      window.localStorage.setItem(STORAGE_KEY, value ? '1' : '0');
    } catch {
      // Storage unavailable (private mode etc.) — non-fatal.
    }
  });
}
