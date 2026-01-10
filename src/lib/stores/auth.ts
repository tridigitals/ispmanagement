/**
 * Authentication Store
 * Manages user authentication state
 */
import { writable, derived, get } from 'svelte/store';
import { auth, type User, type AuthResponse } from '$lib/api/client';
import { appSettings } from './settings';
import { appLogo } from './logo';

// Token storage key
const TOKEN_KEY = 'auth_token';
const USER_KEY = 'auth_user';

// Auth version counter - increments on each refresh to force reactivity
export const authVersion = writable(0);

// Get stored values (check local then session)
function getStoredToken(): string | null {
    if (typeof window === 'undefined') return null;
    const local = localStorage.getItem(TOKEN_KEY);
    const session = sessionStorage.getItem(TOKEN_KEY);
    // console.log('[Auth] Reading token:', { local: !!local, session: !!session });
    return local || session;
}

function getStoredUser(): User | null {
    if (typeof window === 'undefined') return null;
    const stored = localStorage.getItem(USER_KEY) || sessionStorage.getItem(USER_KEY);
    return stored ? JSON.parse(stored) : null;
}

// Create stores
export const token = writable<string | null>(getStoredToken());
export const user = writable<User | null>(getStoredUser());
export const isAuthenticated = derived(token, $token => !!$token);

// isAdmin is now permission-based: checks for 'admin:access' permission OR wildcard '*'
export const isAdmin = derived(user, $user => {
    if (!$user) return false;
    if ($user.is_super_admin) return true;
    // Check for admin access permission or wildcard
    return $user.permissions?.includes('admin:access') || $user.permissions?.includes('*');
});

export const isSuperAdmin = derived(user, $user => ($user as any)?.is_super_admin === true);

// Permission helper
export const can = derived(user, $user => {
    return (action: string, resource: string) => {
        if (!$user) return false;
        // Super admins or Owners typically bypass, but let's stick to permission list
        if ($user.is_super_admin) return true;

        // Explicitly allow Owner role to bypass permission checks
        if ($user.role === 'Owner' || $user.role === 'owner') return true;

        // Check for specific permission "resource:action" or wildcard "resource:*"
        const perm = `${resource}:${action}`;
        const wildcard = `${resource}:*`;
        return $user.permissions?.includes(perm) || $user.permissions?.includes(wildcard) || $user.permissions?.includes('*');
    };
});

// Reactively update logo and settings when token changes
token.subscribe(value => {
    if (typeof window !== 'undefined') {
        // Only refresh logo and settings when logging IN (token exists)
        // On logout (value is null), we keep the cached logo in localStorage
        if (value) {
            // console.log(`[AuthStore] Token exists, refreshing logo and settings...`);
            appLogo.refresh(value);
            appSettings.refresh();
        }
        // When logging out, we intentionally do NOT refresh logo/settings
        // This keeps the last tenant's branding visible on login page
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
user.subscribe(value => {
    if (typeof window === 'undefined') return;
    const storage = getActiveStorage();
    if (storage && value) {
        storage.setItem(USER_KEY, JSON.stringify(value));
    } else if (storage && !value) {
        storage.removeItem(USER_KEY);
    }
});

// Auth actions
export async function login(email: string, password: string, remember: boolean = true): Promise<AuthResponse> {
    console.log('[Auth] Login called. Remember:', remember);
    const response = await auth.login(email, password);
    if (response.token) {
        setAuth(response.token, response.user, remember);
    }
    return response;
}

export async function register(email: string, password: string, name: string): Promise<AuthResponse> {
    const response = await auth.register(email, password, name);
    // Default to remember=true for registration, or could be passed
    if (response.token) {
        setAuth(response.token, response.user, true);
    }
    return response;
}

function setAuth(newToken: string, newUser: User, remember: boolean) {
    console.log('[Auth] Setting auth data. Remember:', remember);
    token.set(newToken);
    user.set(newUser);

    if (typeof window === 'undefined') return;

    // Clear both first to ensure no duplicates
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(USER_KEY);
    sessionStorage.removeItem(TOKEN_KEY);
    sessionStorage.removeItem(USER_KEY);

    const storage = remember ? localStorage : sessionStorage;
    console.log('[Auth] Using storage:', remember ? 'localStorage' : 'sessionStorage');

    storage.setItem(TOKEN_KEY, newToken);
    storage.setItem(USER_KEY, JSON.stringify(newUser));
}

export function logout(): void {
    token.set(null);
    user.set(null);

    if (typeof window === 'undefined') return;
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(USER_KEY);
    sessionStorage.removeItem(TOKEN_KEY);
    sessionStorage.removeItem(USER_KEY);
}

export async function checkAuth(): Promise<boolean> {
    const currentToken = getStoredToken();
    if (!currentToken) return false;

    try {
        const isValid = await auth.validateToken(currentToken);
        if (!isValid) {
            console.warn('[Auth] Token validation returned false');
            logout();
            return false;
        }

        // Refresh user data from backend
        const currentUser = await auth.getCurrentUser(currentToken);
        user.set(currentUser);

        // Increment auth version to force reactive components to re-render
        authVersion.update(v => v + 1);

        // Also update storage so Sidebar gets fresh data
        const storage = localStorage.getItem(TOKEN_KEY) ? localStorage : sessionStorage;
        if (currentUser) {
            storage.setItem(USER_KEY, JSON.stringify(currentUser));
        }

        return true;
    } catch (e) {
        console.warn('[Auth] checkAuth failed (user likely session expired):', e);
        logout();
        return false;
    }
}

// Get current token (for API calls)
export function getToken(): string | null {
    return getStoredToken();
}
