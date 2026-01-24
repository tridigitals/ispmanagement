/**
 * Tauri API Client
 * Wrapper for all backend commands
 */
import { invoke } from '@tauri-apps/api/core';

// Safe invoke wrapper for browser environment
async function safeInvoke<T>(command: string, args?: any): Promise<T> {
    try {
        // Check if we should force remote API usage (for secure client-server deployment)
        const forceRemote = import.meta.env.VITE_USE_REMOTE_API === 'true';

        // Check if running in Tauri AND not forced to use remote
        // @ts-ignore
        if (!forceRemote && typeof window !== 'undefined' && window.__TAURI_INTERNALS__) {
            // console.log(`[Tauri] Invoking ${command}`, args); 
            // if (command === 'get_logo') {
            //    console.log(`[Tauri] get_logo called with token:`, args?.token ? 'YES (Hidden)' : 'NO');
            // }
            return await invoke(command, args);
        }

        // Web Environment (HTTP)
        const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';

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
            // Super Admin
            'list_tenants': { method: 'GET', path: '/superadmin/tenants' },
            'create_tenant': { method: 'POST', path: '/superadmin/tenants' },
            'delete_tenant': { method: 'DELETE', path: '/superadmin/tenants/:id' },
            'list_audit_logs': { method: 'GET', path: '/superadmin/audit-logs' },
            'get_system_health': { method: 'GET', path: '/superadmin/system' },
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
            // Team
            'list_team_members': { method: 'GET', path: '/team' },
            'add_team_member': { method: 'POST', path: '/team' },
            'update_team_member_role': { method: 'PUT', path: '/team/:id' },
            'remove_team_member': { method: 'DELETE', path: '/team/:id' },
            // Roles
            'get_roles': { method: 'GET', path: '/roles' },
            'get_role': { method: 'GET', path: '/roles/:id' },
            'create_new_role': { method: 'POST', path: '/roles' },
            'update_existing_role': { method: 'PUT', path: '/roles/:id' },
            'delete_existing_role': { method: 'DELETE', path: '/roles/:id' },
            'get_permissions': { method: 'GET', path: '/permissions' },
            // Public
            'get_tenant_by_slug': { method: 'GET', path: '/public/tenants/:slug' },
            'get_tenant_by_domain': { method: 'GET', path: '/public/domains/:domain' },
            'get_app_version': { method: 'GET', path: '/version' },
            // Plans
            'list_plans': { method: 'GET', path: '/plans' },
            'get_plan': { method: 'GET', path: '/plans/:plan_id' },
            'create_plan': { method: 'POST', path: '/plans' },
            'update_plan': { method: 'PUT', path: '/plans/:plan_id' },
            'delete_plan': { method: 'DELETE', path: '/plans/:plan_id' },
            'list_features': { method: 'GET', path: '/plans/features' },
            'create_feature': { method: 'POST', path: '/plans/features' },
            'delete_feature': { method: 'DELETE', path: '/plans/features/:feature_id' },
            'set_plan_feature': { method: 'POST', path: '/plans/:plan_id/features' },
            'get_tenant_subscription': { method: 'GET', path: '/plans/subscriptions/:tenant_id' },
            'get_tenant_subscription_details': { method: 'GET', path: '/plans/subscriptions/details' },
            'assign_plan_to_tenant': { method: 'POST', path: '/plans/subscriptions/:tenant_id/assign' },
            'check_feature_access': { method: 'GET', path: '/plans/access/:tenant_id/:feature_code' },
            'send_test': { method: 'POST', path: '/notifications/test' },

            // Storage
            'list_files_admin': { method: 'GET', path: '/storage/files' },
            'list_files_tenant': { method: 'GET', path: '/storage/files' },
            'delete_file_admin': { method: 'DELETE', path: '/storage/files/:file_id' },
            'delete_file_tenant': { method: 'DELETE', path: '/storage/files/:file_id' },
            'upload_init': { method: 'POST', path: '/storage/upload/init' },
            'upload_chunk': { method: 'POST', path: '/storage/upload/chunk' },
            'upload_complete': { method: 'POST', path: '/storage/upload/complete' },
            // Payment
            'list_bank_accounts': { method: 'GET', path: '/payment/banks' },
            'create_bank_account': { method: 'POST', path: '/payment/banks' },
            'delete_bank_account': { method: 'DELETE', path: '/payment/banks/:id' },
            'create_invoice_for_plan': { method: 'POST', path: '/payment/invoices/plan' },
            'get_invoice': { method: 'GET', path: '/payment/invoices/:id' },
            'list_invoices': { method: 'GET', path: '/payment/invoices' },
            'list_all_invoices': { method: 'GET', path: '/payment/invoices/all' },
            'pay_invoice_midtrans': { method: 'POST', path: '/payment/invoices/:id/midtrans' },
            'check_payment_status': { method: 'GET', path: '/payment/invoices/:id/status' },

            // Notifications
            'list_notifications': { method: 'GET', path: '/notifications' },
            'get_unread_count': { method: 'GET', path: '/notifications/unread-count' },
            'mark_as_read': { method: 'POST', path: '/notifications/:id/read' },
            'mark_all_as_read': { method: 'POST', path: '/notifications/read-all' },
            'delete_notification': { method: 'DELETE', path: '/notifications/:id' },
            'get_preferences': { method: 'GET', path: '/notifications/preferences' },
            'update_preference': { method: 'PUT', path: '/notifications/preferences' },
            'subscribe_push': { method: 'POST', path: '/notifications/push/subscribe' },
            'unsubscribe_push': { method: 'POST', path: '/notifications/push/unsubscribe' },
            'send_test_notification': { method: 'POST', path: '/notifications/test' },

            // Tenant
            'get_current_tenant': { method: 'GET', path: '/tenant/me' },
            'update_current_tenant': { method: 'PUT', path: '/tenant/me' },
        };

        let route = commandMap[command];
        if (route) {
            // Helper to convert camelCase to snake_case
            const toSnakeCase = (str: string) => str.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`);

            // Handle path parameters (e.g. :key, :id)
            let path = route.path;
            const queryParams: Record<string, string> = {};

            if (args) {
                for (const [key, value] of Object.entries(args)) {
                    // Skip null/undefined values to avoid 'null' string in query params
                    if (value === null || value === undefined) continue;

                    // Try both camelCase and snake_case for path params
                    const snakeKey = toSnakeCase(key);
                    if (path.includes(`:${key}`)) {
                        path = path.replace(`:${key}`, String(value));
                    } else if (path.includes(`:${snakeKey}`)) {
                        path = path.replace(`:${snakeKey}`, String(value));
                    } else if (route.method === 'GET' && key !== 'token') {
                        // Add non-path params as query params for GET requests (use snake_case for HTTP)
                        queryParams[snakeKey] = String(value);
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
    } catch (error: any) {
        // Downgrade 401/Invalid token errors to warnings as they are handled by the app (logout)
        const isAuthError = error.message?.includes('401') ||
            error.message?.includes('Invalid token') ||
            error.message?.includes('Unauthorized');

        if (isAuthError) {
            console.warn(`API Warning (${command}):`, error.message);
        } else {
            console.error(`API Error (${command}):`, error);
        }
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
    is_super_admin: boolean;
    avatar_url: string | null;
    is_active: boolean;
    created_at: string;
    permissions: string[];
    tenant_slug?: string;
    tenant_id?: string;
    tenant_role?: string;
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

export interface Role {
    id: string;
    name: string;
    description: string | null;
    is_system: boolean;
    level: number;
    permissions?: string[]; // Simplified list of "resource:action" strings
}

export interface Permission {
    id: string;
    resource: string;
    action: string;
    description: string | null;
}

export interface TeamMember {
    id: string;
    user_id: string;
    name: string;
    email: string;
    role: string;
    role_id: string | null;
    role_name: string | null;
    is_active: boolean;
    created_at: string;
}

export interface AuditLog {
    id: string;
    user_id: string | null;
    tenant_id: string | null;
    action: string;
    resource: string;
    resource_id: string | null;
    resource_name?: string;
    details: string | null;
    ip_address: string | null;
    created_at: string;
    user_name?: string;
    user_email?: string;
    tenant_name?: string;
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

// Roles API
export const roles = {
    list: (): Promise<Role[]> =>
        safeInvoke('get_roles', { token: getTokenOrThrow() }),

    getPermissions: (): Promise<Permission[]> =>
        safeInvoke('get_permissions', { token: getTokenOrThrow() }),

    get: (id: string): Promise<Role | null> =>
        safeInvoke('get_role', { token: getTokenOrThrow(), id, roleId: id }),

    create: (name: string, description: string | undefined, level: number, permissions: string[]): Promise<Role> =>
        safeInvoke('create_new_role', { token: getTokenOrThrow(), name, description, level, permissions }),

    update: (id: string, name?: string, description?: string, level?: number, permissions?: string[]): Promise<Role> =>
        safeInvoke('update_existing_role', { token: getTokenOrThrow(), id, roleId: id, name, description, level, permissions }),

    delete: (id: string): Promise<boolean> =>
        safeInvoke('delete_existing_role', { token: getTokenOrThrow(), id, roleId: id }),
};

// Team API
export const team = {
    list: (): Promise<TeamMember[]> =>
        safeInvoke('list_team_members', { token: getTokenOrThrow() }),

    add: (email: string, name: string, roleId: string, password?: string): Promise<TeamMember> =>
        safeInvoke('add_team_member', { token: getTokenOrThrow(), email, name, roleId, password }),

    updateRole: (memberId: string, roleId: string): Promise<void> =>
        safeInvoke('update_team_member_role', { token: getTokenOrThrow(), memberId, roleId }),

    remove: (memberId: string): Promise<void> =>
        safeInvoke('remove_team_member', { token: getTokenOrThrow(), memberId }),
};

// Super Admin API
export const superadmin = {
    listTenants: (): Promise<{ data: any[], total: number }> =>
        safeInvoke('list_tenants', { token: getTokenOrThrow() }),

    createTenant: (name: string, slug: string, customDomain: string | null, ownerEmail: string, ownerPassword: string): Promise<any> =>
        safeInvoke('create_tenant', { token: getTokenOrThrow(), name, slug, customDomain, ownerEmail, ownerPassword }),

    deleteTenant: (id: string): Promise<void> =>
        safeInvoke('delete_tenant', { token: getTokenOrThrow(), id }),

    updateTenant: (id: string, name: string, slug: string, customDomain: string | null, isActive: boolean): Promise<any> =>
        safeInvoke('update_tenant', { token: getTokenOrThrow(), id, name, slug, customDomain, isActive }),

    listAuditLogs: (page?: number, perPage?: number, filters?: { user_id?: string, tenant_id?: string, action?: string, date_from?: string, date_to?: string, search?: string }): Promise<PaginatedResponse<AuditLog>> =>
        safeInvoke('list_audit_logs', { token: getTokenOrThrow(), page, perPage, ...filters }),

    getSystemHealth: (): Promise<any> =>
        safeInvoke('get_system_health', { token: getTokenOrThrow() }),
};

// Public API (No Auth)
export const publicApi = {
    getTenant: (slug: string): Promise<any> =>
        safeInvoke('get_tenant_by_slug', { slug }),
    getTenantByDomain: (domain: string): Promise<any> =>
        safeInvoke('get_tenant_by_domain', { domain }),
};

// Settings API
export const settings = {
    getAll: (): Promise<Setting[]> =>
        safeInvoke('get_all_settings', { token: getTokenOrThrow() }),

    getPublicSettings: (): Promise<{
        app_name?: string,
        app_description?: string,
        default_locale?: string,
        maintenance_mode?: boolean,
        maintenance_message?: string,
        payment_midtrans_enabled?: boolean,
        payment_midtrans_client_key?: string,
        payment_midtrans_is_production?: boolean,
        payment_manual_enabled?: boolean
    }> =>
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

    getLogo: (token?: string): Promise<string | null> =>
        safeInvoke('get_logo', { token }),

    delete: (key: string): Promise<void> =>
        safeInvoke('delete_setting', { token: getTokenOrThrow(), key }),

    sendTestEmail: (toEmail: string): Promise<string> =>
        safeInvoke('send_test_email', { token: getTokenOrThrow(), toEmail }),

    getAppVersion: async (): Promise<string> => {
        const res = await safeInvoke('get_app_version');
        if (typeof res === 'object' && res !== null && 'version' in res) {
            return (res as any).version;
        }
        return res as string || '0.0.0';
    },
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

// Plans API (Superadmin only)
export interface TenantSubscriptionDetails {
    plan_name: string;
    plan_slug: string;
    status: string;
    current_period_end: string | null;
    storage_usage: number;
    storage_limit: number | null;
    member_usage: number;
    member_limit: number | null;
}

// Plans API (Superadmin only)
export const plans = {
    list: (): Promise<any[]> =>
        safeInvoke('list_plans', { token: getTokenOrThrow() }),

    get: (planId: string): Promise<any> =>
        safeInvoke('get_plan', { token: getTokenOrThrow(), planId }),

    create: (name: string, slug: string, description?: string, price_monthly?: number, price_yearly?: number, is_active?: boolean, is_default?: boolean, sort_order?: number): Promise<any> =>
        safeInvoke('create_plan', { token: getTokenOrThrow(), name, slug, description, price_monthly, price_yearly, is_active, is_default, sort_order }),

    update: (planId: string, name?: string, slug?: string, description?: string, price_monthly?: number, price_yearly?: number, is_active?: boolean, is_default?: boolean, sort_order?: number): Promise<any> =>
        safeInvoke('update_plan', { token: getTokenOrThrow(), planId, name, slug, description, price_monthly, price_yearly, is_active, is_default, sort_order }),

    delete: (planId: string): Promise<void> =>
        safeInvoke('delete_plan', { token: getTokenOrThrow(), planId }),

    listFeatures: (): Promise<any[]> =>
        safeInvoke('list_features', { token: getTokenOrThrow() }),

    // DEPRECATED: Feature creation is system managed
    createFeature: (code: string, name: string, description?: string, value_type?: string, category?: string, default_value?: string, sort_order?: number): Promise<any> =>
        safeInvoke('create_feature', { token: getTokenOrThrow(), code, name, description, value_type, category, default_value, sort_order }),

    // DEPRECATED: Feature deletion is system managed
    deleteFeature: (featureId: string): Promise<void> =>
        safeInvoke('delete_feature', { token: getTokenOrThrow(), featureId }),

    setPlanFeature: (planId: string, featureId: string, value: string): Promise<void> =>
        safeInvoke('set_plan_feature', { token: getTokenOrThrow(), planId, featureId, value }),

    getSubscription: (tenantId: string): Promise<any> =>
        safeInvoke('get_tenant_subscription', { token: getTokenOrThrow(), tenantId }),

    getSubscriptionDetails: (tenantId?: string): Promise<TenantSubscriptionDetails> =>
        safeInvoke('get_tenant_subscription_details', { token: getTokenOrThrow(), tenantId }),

    assignPlan: (tenantId: string, planId: string): Promise<any> =>
        safeInvoke('assign_plan_to_tenant', { token: getTokenOrThrow(), tenantId, planId }),

    checkAccess: (tenantId: string, featureCode: string): Promise<any> =>
        safeInvoke('check_feature_access', { token: getTokenOrThrow(), tenantId, featureCode }),
};

export const tenant = {
    getSelf: (): Promise<any> =>
        safeInvoke('get_current_tenant', { token: getTokenOrThrow() }),

    updateSelf: (data: { name?: string, customDomain?: string }): Promise<any> =>
        safeInvoke('update_current_tenant', { token: getTokenOrThrow(), name: data.name, customDomain: data.customDomain }),
};

export interface FileRecord {
    id: string;
    tenant_id: string;
    name: string;
    original_name: string;
    path: string;
    size: number;
    content_type: string;
    uploaded_by: string | null;
    created_at: string;
    updated_at: string;
}

export interface BankAccount {
    id: string;
    bank_name: string;
    account_number: string;
    account_holder: string;
    is_active: boolean;
}

export interface Notification {
    id: string;
    user_id: string;
    tenant_id: string | null;
    title: string;
    message: string;
    notification_type: 'info' | 'success' | 'warning' | 'error';
    category: 'system' | 'team' | 'payment' | 'security';
    action_url: string | null;
    is_read: boolean;
    created_at: string;
}

export interface NotificationPreference {
    id: string;
    user_id: string;
    channel: 'in_app' | 'email' | 'push';
    category: 'system' | 'team' | 'payment' | 'security';
    enabled: boolean;
    updated_at: string;
}

export interface Invoice {
    id: string;
    invoice_number: string;
    amount: number;
    status: string; // "pending", "paid", "failed"
    description: string | null;
    due_date: string;
    paid_at: string | null;
    payment_method: string | null;
}

export const payment = {
    listBanks: (): Promise<BankAccount[]> =>
        safeInvoke('list_bank_accounts', { token: getTokenOrThrow() }),

    createBank: (bank_name: string, account_number: string, account_holder: string): Promise<BankAccount> =>
        safeInvoke('create_bank_account', { token: getTokenOrThrow(), bankName: bank_name, accountNumber: account_number, accountHolder: account_holder }),

    deleteBank: (id: string): Promise<void> =>
        safeInvoke('delete_bank_account', { token: getTokenOrThrow(), id }),

    // Invoice & Transaction
    createInvoiceForPlan: (planId: string, billingCycle: "monthly" | "yearly"): Promise<Invoice> =>
        safeInvoke('create_invoice_for_plan', { token: getTokenOrThrow(), planId, billingCycle }),

    getInvoice: (id: string): Promise<Invoice> =>
        safeInvoke('get_invoice', { token: getTokenOrThrow(), id }),

    listInvoices: (): Promise<Invoice[]> =>
        safeInvoke('list_invoices', { token: getTokenOrThrow() }),

    listAllInvoices: (): Promise<Invoice[]> =>
        safeInvoke('list_all_invoices', { token: getTokenOrThrow() }),

    payMidtrans: (id: string): Promise<string> => // Returns Snap Token
        safeInvoke('pay_invoice_midtrans', { token: getTokenOrThrow(), id }),

    checkStatus: (id: string): Promise<string> =>
        safeInvoke('check_payment_status', { token: getTokenOrThrow(), id }),

    submitPaymentProof: (invoiceId: string, filePath: string): Promise<void> =>
        safeInvoke('submit_payment_proof', { token: getTokenOrThrow(), invoiceId, filePath }),

    verifyPayment: (invoiceId: string, status: "paid" | "failed", rejectionReason?: string): Promise<void> =>
        safeInvoke('verify_payment', { token: getTokenOrThrow(), invoiceId, status, rejectionReason }),
};

export const storage = {
    listFiles: (page: number = 1, perPage: number = 20, search: string = ""): Promise<PaginatedResponse<FileRecord>> =>
        safeInvoke('list_files_admin', { token: getTokenOrThrow(), page, perPage, search: search || null }),

    deleteFile: (fileId: string): Promise<void> =>
        safeInvoke('delete_file_admin', { token: getTokenOrThrow(), fileId }),

    listFilesTenant: (page: number = 1, perPage: number = 20, search: string = ""): Promise<PaginatedResponse<FileRecord>> =>
        safeInvoke('list_files_tenant', { token: getTokenOrThrow(), page, perPage, search: search || null }),

    deleteFileTenant: (fileId: string): Promise<void> =>
        safeInvoke('delete_file_tenant', { token: getTokenOrThrow(), fileId }),

    uploadFile: async (file: File): Promise<FileRecord> => {
        const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';
        const formData = new FormData();
        formData.append('file', file);

        const response = await fetch(`${API_BASE}/storage/upload`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${getTokenOrThrow()}`
            },
            body: formData
        });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(`Upload failed: ${error}`);
        }

        return await response.json();
    },
};

export const notifications = {
    list: (page?: number, perPage?: number): Promise<PaginatedResponse<Notification>> =>
        safeInvoke('list_notifications', { token: getTokenOrThrow(), page, perPage }),

    getUnreadCount: (): Promise<{ count: number }> =>
        safeInvoke('get_unread_count', { token: getTokenOrThrow() }),

    markAsRead: (id: string): Promise<void> =>
        safeInvoke('mark_as_read', { token: getTokenOrThrow(), id }),

    markAllAsRead: (): Promise<void> =>
        safeInvoke('mark_all_as_read', { token: getTokenOrThrow() }),

    delete: (id: string): Promise<void> =>
        safeInvoke('delete_notification', { token: getTokenOrThrow(), id }),

    getPreferences: (): Promise<NotificationPreference[]> =>
        safeInvoke('get_preferences', { token: getTokenOrThrow() }),

    updatePreference: (channel: string, category: string, enabled: boolean): Promise<void> =>
        safeInvoke('update_preference', { token: getTokenOrThrow(), channel, category, enabled }),

    subscribePush: (endpoint: string, p256dh: string, auth: string): Promise<void> =>
        safeInvoke('subscribe_push', { token: getTokenOrThrow(), endpoint, p256dh, auth }),

    unsubscribePush: (endpoint: string): Promise<void> =>
        safeInvoke('unsubscribe_push', { token: getTokenOrThrow(), endpoint }),

    sendTest: (): Promise<void> =>
        safeInvoke('send_test', { token: getTokenOrThrow() }),
};

// Combined API object
export const api = {
    auth,
    users,
    roles,
    team,
    superadmin,
    settings,
    install,
    plans,
    storage,
    payment,
    tenant,
    notifications,
};

export default api;
