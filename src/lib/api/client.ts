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
        if (typeof window !== 'undefined' && !window.__TAURI_INTERNALS__) {
            console.warn(`[Mock] Calling ${command} with`, args);
            // Return mock data or throw error depending on need
            if (command === 'get_current_user') return null as any;
            if (command === 'get_all_settings') return [] as any;
            if (command === 'list_users') return { data: [], total: 0, page: 1, per_page: 10 } as any;
            throw new Error(`Tauri API not available (Browser Environment). Mocking ${command}.`);
        }
        return await invoke(command, args);
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
    token: string;
    expires_at: string;
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
};

// Install API
export const install = {
    checkIsInstalled: (): Promise<boolean> =>
        safeInvoke('is_installed'),

    installApp: (adminName: string, adminEmail: string, adminPassword: string): Promise<User> =>
        safeInvoke('install_app', {
            adminName,
            adminEmail,
            adminPassword
        }),
};

// Combined API object
export const api = {
    auth,
    users,
    settings,
    install,
};

export default api;
