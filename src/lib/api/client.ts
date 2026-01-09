/**
 * Tauri API Client
 * Wrapper for all backend commands
 */
import { invoke } from '@tauri-apps/api/core';

// Safe invoke wrapper for browser environment
async function safeInvoke<T>(command: string, args?: any): Promise<T> {
    try {
        // Check if running in Tauri
        // @ts-ignore
        if (typeof window !== 'undefined' && window.__TAURI_INTERNALS__) {
            return await invoke(command, args);
        }

        // Web Environment (HTTP)
        const API_BASE = 'http://localhost:3000/api';

        // Map commands to API endpoints
        const commandMap: Record<string, { method: string, path: string }> = {
            // Install
            'is_installed': { method: 'GET', path: '/install/check' },
            'install_app': { method: 'POST', path: '/install' },
            // Auth
            'login': { method: 'POST', path: '/auth/login' },
            'register': { method: 'POST', path: '/auth/register' },
            'verify_email': { method: 'POST', path: '/auth/verify-email' },
            'forgot_password': { method: 'POST', path: '/auth/forgot-password' },
            'reset_password': { method: 'POST', path: '/auth/reset-password' },
            'validate_token': { method: 'POST', path: '/auth/validate' },
            'get_auth_settings': { method: 'GET', path: '/auth/settings' },
            'get_current_user': { method: 'GET', path: '/auth/me' },
            // Users
            'list_users': { method: 'GET', path: '/users' },
            'get_user': { method: 'GET', path: '/users/:id' },
            'create_user': { method: 'POST', path: '/users' },
            'update_user': { method: 'PUT', path: '/users/:id' },
            'delete_user': { method: 'DELETE', path: '/users/:id' },
            // Settings
            'get_logo': { method: 'GET', path: '/settings/logo' },
            'get_all_settings': { method: 'GET', path: '/settings' },
            'get_public_settings': { method: 'GET', path: '/settings/public' },
            'upsert_setting': { method: 'POST', path: '/settings' },
            'get_setting': { method: 'GET', path: '/settings/:key' },
            'get_setting_value': { method: 'GET', path: '/settings/:key/value' },
            'delete_setting': { method: 'DELETE', path: '/settings/:key' },
            'upload_logo': { method: 'POST', path: '/settings/logo' },
            'send_test_email': { method: 'POST', path: '/settings/test-email' },
        };

        let route = commandMap[command];
        if (route) {
            // Handle path parameters (e.g. :key, :id)
            let path = route.path;
            const queryParams: Record<string, string> = {};

            if (args) {
                for (const [key, value] of Object.entries(args)) {
                    if (path.includes(`:${key}`)) {
                        path = path.replace(`:${key}`, String(value));
                    } else if (route.method === 'GET' && key !== 'token') {
                        // Add non-path params as query params for GET requests
                        queryParams[key] = String(value);
                    }
                }
            }

            // Build query string for GET requests
            const queryString = Object.keys(queryParams).length > 0
                ? '?' + new URLSearchParams(queryParams).toString()
                : '';

            const headers: Record<string, string> = {
                'Content-Type': 'application/json',
            };

            // Add token if available in args or storage
            const token = args?.token || localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
            if (token) {
                headers['Authorization'] = `Bearer ${token}`;
            }

            const response = await fetch(`${API_BASE}${path}${queryString}`, {
                method: route.method,
                headers,
                body: route.method !== 'GET' ? JSON.stringify(args || {}) : undefined,
            });

            if (!response.ok) {
                const errorBody = await response.json().catch(() => ({}));
                throw new Error(errorBody.error || `HTTP Error ${response.status}`);
            }

            return await response.json();
        }

        console.warn(`[Mock] Calling ${command} with`, args);
        // Return mock data for unimplemented endpoints
        if (command === 'get_current_user') return null as any;
        if (command === 'get_all_settings') return [] as any;
        if (command === 'list_users') return { data: [], total: 0, page: 1, per_page: 10 } as any;
        if (command === 'validate_token') return true as any;
        if (command === 'is_installed') return false as any; // DEFAULT TO FALSE SO WE DON'T BYPASS INSTALL
        if (command === 'get_auth_settings') return {
            jwt_expiry_hours: 24,
            password_min_length: 8,
            password_require_uppercase: true,
            password_require_number: true,
            password_require_special: false,
            max_login_attempts: 5,
            lockout_duration_minutes: 15,
            allow_registration: true
        } as any;

        throw new Error(`Command '${command}' not implemented in HTTP API yet.`);
    } catch (error) {
        console.error(`API Error (${command}):`, error);
        throw error;
    }
}

// Helper to get token
function getTokenOrThrow(): string {
    if (typeof window === 'undefined') throw new Error("Client side only");
    const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    if (!token) throw new Error("Authentication required");
    return token;
}

// Types
export interface User {
    id: string;
    email: string;
    name: string;
    role: string;
    avatar_url: string | null;
    is_active: boolean;
    created_at: string;
}

export interface AuthResponse {
    user: User;
    token?: string;
    expires_at?: string;
    message?: string;
}

export interface PaginatedResponse<T> {
    data: T[];
    total: number;
    page: number;
    per_page: number;
}

export interface Setting {
    id: string;
    key: string;
    value: string;
    description: string | null;
    created_at: string;
    updated_at: string;
}

export interface AuthSettings {
    jwt_expiry_hours: number;
    password_min_length: number;
    password_require_uppercase: boolean;
    password_require_number: boolean;
    password_require_special: boolean;
    max_login_attempts: number;
    lockout_duration_minutes: number;
    allow_registration: boolean;
}

// Auth API
export const auth = {
    register: (email: string, password: string, name: string): Promise<AuthResponse> =>
        safeInvoke('register', { email, password, name }),

    login: (email: string, password: string): Promise<AuthResponse> =>
        safeInvoke('login', { email, password }),

    logout: (token: string): Promise<void> =>
        safeInvoke('logout', { token }),

    changePassword: (token: string, oldPassword: string, newPassword: string): Promise<void> =>
        safeInvoke('change_password', { token, old_password: oldPassword, new_password: newPassword }),

    getCurrentUser: (token: string): Promise<User> =>
        safeInvoke('get_current_user', { token }),

    validateToken: (token: string): Promise<boolean> =>
        safeInvoke('validate_token', { token }),

    verifyEmail: (token: string): Promise<AuthResponse> =>
        safeInvoke('verify_email', { token }),

    forgotPassword: (email: string): Promise<void> =>
        safeInvoke('forgot_password', { email }),

    resetPassword: (token: string, password: string): Promise<void> =>
        safeInvoke('reset_password', { token, password }),
};

// Users API
export const users = {
    list: (page?: number, perPage?: number): Promise<PaginatedResponse<User>> =>
        safeInvoke('list_users', { token: getTokenOrThrow(), page, perPage }),

    get: (id: string): Promise<User> =>
        safeInvoke('get_user', { token: getTokenOrThrow(), id }),

    create: (email: string, password: string, name: string): Promise<User> =>
        safeInvoke('create_user', { token: getTokenOrThrow(), email, password, name }),

    update: (id: string, data: {
        email?: string;
        name?: string;
        role?: string;
        isActive?: boolean;
    }): Promise<User> =>
        safeInvoke('update_user', {
            token: getTokenOrThrow(),
            id,
            email: data.email,
            name: data.name,
            role: data.role,
            is_active: data.isActive,
        }),

    delete: (id: string): Promise<void> =>
        safeInvoke('delete_user', { token: getTokenOrThrow(), id }),
};

// Settings API
export const settings = {
    getAll: (): Promise<Setting[]> =>
        safeInvoke('get_all_settings', { token: getTokenOrThrow() }),

    getPublicSettings: (): Promise<{ app_name?: string, app_description?: string, default_locale?: string }> =>
        safeInvoke('get_public_settings'),

    getAuthSettings: (): Promise<AuthSettings> =>
        safeInvoke('get_auth_settings'),

    get: (key: string): Promise<Setting | null> =>
        safeInvoke('get_setting', { token: getTokenOrThrow(), key }),

    getValue: (key: string): Promise<string | null> =>
        safeInvoke('get_setting_value', { token: getTokenOrThrow(), key }),

    upsert: (key: string, value: string, description?: string): Promise<Setting> =>
        safeInvoke('upsert_setting', { token: getTokenOrThrow(), key, value, description }),

    uploadLogo: (fileBase64: string): Promise<string> =>
        safeInvoke('upload_logo', { token: getTokenOrThrow(), content: fileBase64 }),

    getLogo: (): Promise<string | null> =>
        safeInvoke('get_logo'),

    delete: (key: string): Promise<void> =>
        safeInvoke('delete_setting', { token: getTokenOrThrow(), key }),

    sendTestEmail: (toEmail: string): Promise<string> =>
        safeInvoke('send_test_email', { token: getTokenOrThrow(), toEmail }),
};

// Install API
export const install = {
    checkIsInstalled: async (): Promise<boolean> => {
        const res = await safeInvoke('is_installed');
        // Handle object response from HTTP API ({ installed: boolean })
        if (typeof res === 'object' && res !== null && 'installed' in res) {
            return (res as any).installed;
        }
        return res as boolean;
    },

    installApp: async (
        adminName: string,
        adminEmail: string,
        adminPassword: string,
        appName?: string,
        appUrl?: string
    ): Promise<User> => {
        const res = await safeInvoke('install_app', {
            adminName,
            adminEmail,
            adminPassword,
            appName,
            appUrl
        });

        // Handle object response from HTTP API ({ user: User, ... })
        if (typeof res === 'object' && res !== null && 'user' in res) {
            return (res as any).user;
        }
        return res as User;
    },
};

// Combined API object
export const api = {
    auth,
    users,
    settings,
    install,
};

export default api;
