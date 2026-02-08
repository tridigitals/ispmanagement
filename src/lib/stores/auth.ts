/**
 * Authentication Store
 * Manages user authentication state
 */
import { writable, derived, get } from 'svelte/store';
import { api, auth, type User, type Tenant, type AuthResponse } from '$lib/api/client';
import { appSettings } from './settings';
import { appLogo } from './logo';

// Tracks whether backend/API is reachable. We keep sessions during transient outages.
export const backendAvailable = writable(true);

// Token storage key
const TOKEN_KEY = 'auth_token';
const USER_KEY = 'auth_user';
const TENANT_KEY = 'auth_tenant';

// Auth version counter - increments on each refresh to force reactivity
export const authVersion = writable(0);

// Get stored values (check local then session)
function getStoredToken(): string | null {
  if (typeof window === 'undefined') return null;
  const local = localStorage.getItem(TOKEN_KEY);
  const session = sessionStorage.getItem(TOKEN_KEY);
  return local || session;
}

function getStoredUser(): User | null {
  if (typeof window === 'undefined') return null;
  const stored = localStorage.getItem(USER_KEY) || sessionStorage.getItem(USER_KEY);
  return stored ? JSON.parse(stored) : null;
}

function getStoredTenant(): Tenant | null {
  if (typeof window === 'undefined') return null;
  const stored = localStorage.getItem(TENANT_KEY) || sessionStorage.getItem(TENANT_KEY);
  return stored ? JSON.parse(stored) : null;
}

// Create stores
export const token = writable<string | null>(getStoredToken());
export const user = writable<User | null>(getStoredUser());
export const tenant = writable<Tenant | null>(getStoredTenant());
export const isAuthenticated = derived(token, ($token) => !!$token);

// Derived store to check if 2FA setup is required by tenant but not enabled by user
export const is2FARequiredButDisabled = derived([user, tenant], ([$user, $tenant]) => {
  if (!$user || !$tenant) return false;
  // Super Admins bypass enforcement
  if ($user.is_super_admin) return false;
  return $tenant.enforce_2fa && !$user.two_factor_enabled;
});

// isAdmin is now permission-based: checks for 'admin:access' permission OR wildcard '*'
export const isAdmin = derived(user, ($user) => {
  if (!$user) return false;
  if ($user.is_super_admin) return true;
  // Check for admin access permission or wildcard
  return $user.permissions?.includes('admin:access') || $user.permissions?.includes('*');
});

export const isSuperAdmin = derived(user, ($user) => ($user as any)?.is_super_admin === true);

// Permission helper
export const can = derived(user, ($user) => {
  return (action: string, resource: string) => {
    if (!$user) return false;
    // Super admins or Owners typically bypass, but let's stick to permission list
    if ($user.is_super_admin) return true;

    // Explicitly allow Owner role to bypass permission checks
    if ($user.role === 'Owner' || $user.role === 'owner') return true;

    // Check for specific permission "resource:action" or wildcard "resource:*"
    const perm = `${resource}:${action}`;
    const wildcard = `${resource}:*`;
    return (
      $user.permissions?.includes(perm) ||
      $user.permissions?.includes(wildcard) ||
      $user.permissions?.includes('*')
    );
  };
});

// Reactively update logo and settings when token changes
token.subscribe((value) => {
  if (typeof window !== 'undefined') {
    // Only refresh logo and settings when logging IN (token exists)
    if (value) {
      appLogo.refresh(value);
      appSettings.refresh();
    } else {
      // On logout, reset settings to default (secure by default)
      appSettings.reset();
    }
  }
});

// Helper to determine active storage
function getActiveStorage(): Storage | null {
  if (typeof window === 'undefined') return null;
  if (localStorage.getItem(TOKEN_KEY)) return localStorage;
  if (sessionStorage.getItem(TOKEN_KEY)) return sessionStorage;
  return null; // Not logged in or no storage
}

// Persist user changes to the active storage
user.subscribe((value) => {
  if (typeof window === 'undefined') return;
  const storage = getActiveStorage();
  if (storage && value) {
    storage.setItem(USER_KEY, JSON.stringify(value));
  } else if (storage && !value) {
    storage.removeItem(USER_KEY);
  }
});

// Persist tenant changes to active storage
tenant.subscribe((value) => {
  if (typeof window === 'undefined') return;
  const storage = getActiveStorage();
  if (storage && value) {
    storage.setItem(TENANT_KEY, JSON.stringify(value));
  } else if (storage && !value) {
    storage.removeItem(TENANT_KEY);
  }
});

// Auth actions
export async function login(
  email: string,
  password: string,
  remember: boolean = true,
): Promise<AuthResponse> {
  const response = await auth.login(email, password);
  if (response.token) {
    setAuthData(response.token, response.user, remember, response.tenant);
  }
  return response;
}

export async function register(
  email: string,
  password: string,
  name: string,
): Promise<AuthResponse> {
  const response = await auth.register(email, password, name);
  // Default to remember=true for registration, or could be passed
  if (response.token) {
    setAuthData(response.token, response.user, true, response.tenant);
  }
  return response;
}

export function setAuthData(
  newToken: string,
  newUser: User,
  remember: boolean,
  newTenant?: Tenant,
) {
  token.set(newToken);
  user.set(newUser);
  if (newTenant) tenant.set(newTenant);

  if (typeof window === 'undefined') return;

  // Clear both first to ensure no duplicates
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(USER_KEY);
  localStorage.removeItem(TENANT_KEY);
  sessionStorage.removeItem(TOKEN_KEY);
  sessionStorage.removeItem(USER_KEY);
  sessionStorage.removeItem(TENANT_KEY);

  const storage = remember ? localStorage : sessionStorage;

  storage.setItem(TOKEN_KEY, newToken);
  storage.setItem(USER_KEY, JSON.stringify(newUser));
  if (newTenant) {
    storage.setItem(TENANT_KEY, JSON.stringify(newTenant));
  }
}

export function logout(): void {
  token.set(null);
  user.set(null);
  tenant.set(null);

  if (typeof window === 'undefined') return;
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(USER_KEY);
  localStorage.removeItem(TENANT_KEY);
  sessionStorage.removeItem(TOKEN_KEY);
  sessionStorage.removeItem(USER_KEY);
  sessionStorage.removeItem(TENANT_KEY);
}

export async function checkAuth(): Promise<boolean> {
  const currentToken = getStoredToken();
  if (!currentToken) return false;

  try {
    backendAvailable.set(true);

    const isUnauthorizedError = (e: unknown) => {
      const msg = String((e as any)?.message || e || '');
      return (
        msg.includes('HTTP Error 401') ||
        msg.includes('HTTP Error 403') ||
        msg.toLowerCase().includes('unauthorized') ||
        msg.toLowerCase().includes('invalid token')
      );
    };

    // Validate token: on network/server errors we keep the local session and retry later.
    // Only force logout if backend explicitly rejects the token.
    let isValid = true;
    try {
      const res: any = await auth.validateToken(currentToken);
      isValid = typeof res === 'boolean' ? res : res?.valid === true;
    } catch (e) {
      if (isUnauthorizedError(e)) {
        logout();
        return false;
      }
      backendAvailable.set(false);
      // Keep existing auth state during outage.
      return !!getStoredUser();
    }

    if (!isValid) {
      logout();
      return false;
    }

    // Refresh user data from backend
    try {
      const currentUser = await auth.getCurrentUser(currentToken);
      user.set(currentUser);
    } catch (e) {
      if (isUnauthorizedError(e)) {
        logout();
        return false;
      }
      backendAvailable.set(false);
      // Keep existing user/tenant data; we'll refresh when backend is back.
      return !!getStoredUser();
    }

    // Fetch current tenant info if logged in
    try {
      const currentTenant = await api.tenant.getSelf();
      tenant.set(currentTenant);
    } catch (e) {
      // Ignore if tenant fetch fails (e.g. no tenant assigned yet)
    }

    // Increment auth version to force reactive components to re-render
    authVersion.update((v) => v + 1);

    // Also update storage so components get fresh data
    const storage = localStorage.getItem(TOKEN_KEY) ? localStorage : sessionStorage;
    const refreshedUser = get(user);
    if (refreshedUser) storage.setItem(USER_KEY, JSON.stringify(refreshedUser));
    const $tenant = get(tenant);
    if ($tenant) {
      storage.setItem(TENANT_KEY, JSON.stringify($tenant));
    }

    return true;
  } catch (e) {
    // Final safety net: only logout on explicit auth failures.
    const msg = String((e as any)?.message || e || '');
    const isAuthError =
      msg.includes('HTTP Error 401') ||
      msg.includes('HTTP Error 403') ||
      msg.toLowerCase().includes('unauthorized') ||
      msg.toLowerCase().includes('invalid token');

    if (isAuthError) {
      logout();
      return false;
    }

    backendAvailable.set(false);
    return !!getStoredUser();
  }
}

// Get current token (for API calls)
export function getToken(): string | null {
  return getStoredToken();
}
