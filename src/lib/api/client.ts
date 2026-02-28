/**
 * Tauri API Client
 * Wrapper for all backend commands
 */
import { invoke } from '@tauri-apps/api/core';
import { getApiBaseUrl } from '$lib/utils/apiUrl';

const AUTH_STORAGE_KEYS = ['auth_token', 'auth_user', 'auth_tenant', 'active_tenant_slug'] as const;
let authRedirectInProgress = false;

function hasStoredAuthToken(): boolean {
  if (typeof window === 'undefined') return false;
  return !!(localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token'));
}

function clearStoredAuthData() {
  if (typeof window === 'undefined') return;
  for (const key of AUTH_STORAGE_KEYS) {
    localStorage.removeItem(key);
    sessionStorage.removeItem(key);
  }
}

function handleAuthExpired(reason: string) {
  if (typeof window === 'undefined') return;
  // Only auto-redirect when there was an active local session.
  if (!hasStoredAuthToken()) return;
  if (authRedirectInProgress) return;
  authRedirectInProgress = true;

  clearStoredAuthData();

  try {
    window.dispatchEvent(new CustomEvent('app:auth-expired', { detail: { reason } }));
  } catch {
    // non-blocking
  }

  // Give layout/store listeners one tick to handle graceful logout + SPA redirect.
  setTimeout(() => {
    if (window.location.pathname.startsWith('/login')) return;
    window.location.assign('/login?reason=expired');
  }, 0);
}

// Safe invoke wrapper for browser environment
async function safeInvoke<T>(command: string, args?: any): Promise<T> {
  try {
    // Check if we should force remote API usage (for secure client-server deployment)
    const forceRemote = import.meta.env.VITE_USE_REMOTE_API === 'true';
    let looksLikeTauri = false;

    if (typeof window !== 'undefined') {
      const w = window as any;
      looksLikeTauri =
        !!w.__TAURI_INTERNALS__ ||
        !!w.__TAURI__ ||
        (typeof navigator !== 'undefined' &&
          typeof navigator.userAgent === 'string' &&
          navigator.userAgent.toLowerCase().includes('tauri'));
    }

    // Registration must happen in web browser only (tenant/domain-aware flow).
    if (looksLikeTauri && command === 'register') {
      throw new Error(
        'Pendaftaran akun hanya tersedia melalui web browser pada domain/workspace tenant yang benar.',
      );
    }

    // Prefer Tauri IPC when available (prevents hanging HTTP fetch when no server is running)
    if (!forceRemote && typeof window !== 'undefined') {
      if (looksLikeTauri) {
        try {
          return await invoke(command, args);
        } catch (e: any) {
          // If IPC isn't actually available (e.g. running in web), fall back to HTTP
          const msg = String(e?.message || e || '');
          const canFallback =
            msg.includes('ipc') ||
            msg.includes('tauri') ||
            msg.includes('not implemented') ||
            msg.includes('undefined') ||
            msg.includes('not allowed') ||
            msg.includes('not available') ||
            msg.includes('state not managed');
          if (!canFallback) throw e;
        }
      }
    }

    // Web Environment (HTTP)
    const API_BASE = getApiBaseUrl();

    // Map commands to API endpoints
    const commandMap: Record<string, { method: string; path: string }> = {
      // Install
      is_installed: { method: 'GET', path: '/install/check' },
      install_app: { method: 'POST', path: '/install' },
      // Auth
      login: { method: 'POST', path: '/auth/login' },
      register: { method: 'POST', path: '/auth/register' },
      verify_email: { method: 'POST', path: '/auth/verify-email' },
      forgot_password: { method: 'POST', path: '/auth/forgot-password' },
      reset_password: { method: 'POST', path: '/auth/reset-password' },
      validate_token: { method: 'POST', path: '/auth/validate' },
      get_auth_settings: { method: 'GET', path: '/auth/settings' },
      get_current_user: { method: 'GET', path: '/auth/me' },
      enable_2fa: { method: 'POST', path: '/auth/2fa/enable' },
      verify_2fa_setup: { method: 'POST', path: '/auth/2fa/verify-setup' },
      disable_2fa: { method: 'POST', path: '/auth/2fa/disable' },
      verify_login_2fa: { method: 'POST', path: '/auth/2fa/verify' },
      request_email_2fa_setup: { method: 'POST', path: '/auth/2fa/email/enable-request' },
      verify_email_2fa_setup: { method: 'POST', path: '/auth/2fa/email/enable-verify' },
      set_2fa_preference: { method: 'POST', path: '/auth/2fa/preference' },
      request_email_otp: { method: 'POST', path: '/auth/2fa/email/request' },
      verify_email_otp: { method: 'POST', path: '/auth/2fa/email/verify' },
      list_trusted_devices: { method: 'GET', path: '/auth/trusted-devices' },
      revoke_trusted_device: { method: 'DELETE', path: '/auth/trusted-devices/:deviceId' },
      // Users
      list_users: { method: 'GET', path: '/users' },
      get_user: { method: 'GET', path: '/users/:id' },
      create_user: { method: 'POST', path: '/users' },
      update_user: { method: 'PUT', path: '/users/:id' },
      delete_user: { method: 'DELETE', path: '/users/:id' },
      list_my_addresses: { method: 'GET', path: '/users/me/addresses' },
      create_my_address: { method: 'POST', path: '/users/me/addresses' },
      update_my_address: { method: 'PUT', path: '/users/me/addresses/:addressId' },
      delete_my_address: { method: 'DELETE', path: '/users/me/addresses/:addressId' },
      // Super Admin
      list_tenants: { method: 'GET', path: '/superadmin/tenants' },
      create_tenant: { method: 'POST', path: '/superadmin/tenants' },
      delete_tenant: { method: 'DELETE', path: '/superadmin/tenants/:id' },
      list_audit_logs: { method: 'GET', path: '/superadmin/audit-logs' },
      get_system_health: { method: 'GET', path: '/superadmin/system' },
      get_system_diagnostics: { method: 'GET', path: '/superadmin/diagnostics' },

      // Support Tickets
      list_support_tickets: { method: 'GET', path: '/support/tickets' },
      get_support_ticket_stats: { method: 'GET', path: '/support/tickets/stats' },
      create_support_ticket: { method: 'POST', path: '/support/tickets' },
      get_support_ticket: { method: 'GET', path: '/support/tickets/:id' },
      reply_support_ticket: { method: 'POST', path: '/support/tickets/:id/messages' },
      update_support_ticket: { method: 'PUT', path: '/support/tickets/:id' },

      // Customers (tenant scoped)
      list_customers: { method: 'GET', path: '/customers' },
      get_customer: { method: 'GET', path: '/customers/:customerId' },
      create_customer: { method: 'POST', path: '/customers' },
      create_customer_with_portal: { method: 'POST', path: '/customers/with-portal' },
      update_customer: { method: 'PUT', path: '/customers/:customerId' },
      delete_customer: { method: 'DELETE', path: '/customers/:customerId' },
      list_customer_registration_invites: { method: 'GET', path: '/customers/invites' },
      create_customer_registration_invite: { method: 'POST', path: '/customers/invites' },
      get_customer_registration_invite_policy: { method: 'GET', path: '/customers/invites/policy' },
      update_customer_registration_invite_policy: {
        method: 'PUT',
        path: '/customers/invites/policy',
      },
      get_customer_registration_invite_summary: { method: 'GET', path: '/customers/invites/summary' },
      revoke_customer_registration_invite: {
        method: 'DELETE',
        path: '/customers/invites/:inviteId',
      },
      list_customer_locations: { method: 'GET', path: '/customers/:customerId/locations' },
      create_customer_location: { method: 'POST', path: '/customers/locations' },
      update_customer_location: { method: 'PUT', path: '/customers/locations/:locationId' },
      delete_customer_location: { method: 'DELETE', path: '/customers/locations/:locationId' },
      list_customer_portal_users: { method: 'GET', path: '/customers/:customerId/portal-users' },
      add_customer_portal_user: { method: 'POST', path: '/customers/portal-users/add' },
      create_customer_portal_user: { method: 'POST', path: '/customers/portal-users/create' },
      remove_customer_portal_user: {
        method: 'DELETE',
        path: '/customers/portal-users/:customerUserId',
      },
      list_customer_subscriptions: { method: 'GET', path: '/customers/:customerId/subscriptions' },
      create_customer_subscription: {
        method: 'POST',
        path: '/customers/:customerId/subscriptions',
      },
      update_customer_subscription: {
        method: 'PUT',
        path: '/customers/subscriptions/:subscriptionId',
      },
      delete_customer_subscription: {
        method: 'DELETE',
        path: '/customers/subscriptions/:subscriptionId',
      },
      list_my_customer_locations: { method: 'GET', path: '/customers/portal/my-locations' },
      create_my_customer_location: { method: 'POST', path: '/customers/portal/my-locations' },
      list_my_customer_packages: { method: 'GET', path: '/customers/portal/my-packages' },
      list_my_customer_subscriptions: { method: 'GET', path: '/customers/portal/my-subscriptions' },
      create_my_customer_subscription_invoice: {
        method: 'POST',
        path: '/customers/portal/checkout',
      },
      list_installation_work_orders: { method: 'GET', path: '/admin/work-orders' },
      assign_installation_work_order: {
        method: 'POST',
        path: '/admin/work-orders/:id/assign',
      },
      start_installation_work_order: {
        method: 'POST',
        path: '/admin/work-orders/:id/start',
      },
      complete_installation_work_order: {
        method: 'POST',
        path: '/admin/work-orders/:id/complete',
      },
      cancel_installation_work_order: {
        method: 'POST',
        path: '/admin/work-orders/:id/cancel',
      },
      // Settings
      get_logo: { method: 'GET', path: '/settings/logo' },
      get_all_settings: { method: 'GET', path: '/settings' },
      get_public_settings: { method: 'GET', path: '/settings/public' },
      get_email_verification_readiness: {
        method: 'GET',
        path: '/settings/email-verification-readiness',
      },
      upsert_setting: { method: 'POST', path: '/settings' },
      get_setting: { method: 'GET', path: '/settings/:key' },
      get_setting_value: { method: 'GET', path: '/settings/:key/value' },
      delete_setting: { method: 'DELETE', path: '/settings/:key' },
      upload_logo: { method: 'POST', path: '/settings/logo' },
      send_test_email: { method: 'POST', path: '/settings/test-email' },
      test_smtp_connection: { method: 'POST', path: '/settings/test-smtp' },
      // Team
      list_team_members: { method: 'GET', path: '/team' },
      add_team_member: { method: 'POST', path: '/team' },
      // Use :memberId so old payloads that only send `memberId` still resolve correctly.
      update_team_member_role: { method: 'PUT', path: '/team/:memberId' },
      remove_team_member: { method: 'DELETE', path: '/team/:memberId' },
      // Roles
      get_roles: { method: 'GET', path: '/roles' },
      get_role: { method: 'GET', path: '/roles/:id' },
      create_new_role: { method: 'POST', path: '/roles' },
      update_existing_role: { method: 'PUT', path: '/roles/:id' },
      delete_existing_role: { method: 'DELETE', path: '/roles/:id' },
      get_permissions: { method: 'GET', path: '/permissions' },
      // Public
      get_tenant_by_slug: { method: 'GET', path: '/public/tenants/:slug' },
      get_tenant_by_domain: { method: 'GET', path: '/public/domains/:domain' },
      get_customer_registration_status_by_domain: {
        method: 'GET',
        path: '/public/customer-registration-status',
      },
      validate_customer_registration_invite_by_domain: {
        method: 'GET',
        path: '/public/customer-invite/validate',
      },
      register_customer_by_domain: { method: 'POST', path: '/public/customer-register' },
      get_app_version: { method: 'GET', path: '/version' },
      // Plans
      list_plans: { method: 'GET', path: '/plans' },
      get_plan: { method: 'GET', path: '/plans/:plan_id' },
      create_plan: { method: 'POST', path: '/plans' },
      update_plan: { method: 'PUT', path: '/plans/:plan_id' },
      delete_plan: { method: 'DELETE', path: '/plans/:plan_id' },
      list_features: { method: 'GET', path: '/plans/features' },
      create_feature: { method: 'POST', path: '/plans/features' },
      delete_feature: { method: 'DELETE', path: '/plans/features/:feature_id' },
      set_plan_feature: { method: 'POST', path: '/plans/:plan_id/features' },
      get_tenant_subscription: { method: 'GET', path: '/plans/subscriptions/:tenant_id' },
      get_tenant_subscription_details: { method: 'GET', path: '/plans/subscriptions/details' },
      assign_plan_to_tenant: { method: 'POST', path: '/plans/subscriptions/:tenant_id/assign' },
      check_feature_access: { method: 'GET', path: '/plans/access/:tenant_id/:feature_code' },
      send_test: { method: 'POST', path: '/notifications/test' },

      // Storage
      list_files_admin: { method: 'GET', path: '/storage/files' },
      list_files_tenant: { method: 'GET', path: '/storage/files' },
      delete_file_admin: { method: 'DELETE', path: '/storage/files/:file_id' },
      delete_file_tenant: { method: 'DELETE', path: '/storage/files/:file_id' },
      upload_init: { method: 'POST', path: '/storage/upload/init' },
      upload_chunk: { method: 'POST', path: '/storage/upload/chunk' },
      upload_complete: { method: 'POST', path: '/storage/upload/complete' },
      // Payment
      list_bank_accounts: { method: 'GET', path: '/payment/banks' },
      create_bank_account: { method: 'POST', path: '/payment/banks' },
      delete_bank_account: { method: 'DELETE', path: '/payment/banks/:id' },
      create_invoice_for_plan: { method: 'POST', path: '/payment/invoices/plan' },
      create_invoice_for_customer_subscription: {
        method: 'POST',
        path: '/payment/invoices/customer-package/create',
      },
      generate_due_customer_package_invoices: {
        method: 'POST',
        path: '/payment/invoices/customer-package/generate-due',
      },
      list_billing_collection_logs: { method: 'GET', path: '/payment/billing-collection/logs' },
      list_invoice_reminder_logs: {
        method: 'GET',
        path: '/payment/billing-collection/reminders',
      },
      run_billing_collection_now: { method: 'POST', path: '/payment/billing-collection/run-now' },
      get_invoice: { method: 'GET', path: '/payment/invoices/:id' },
      list_invoices: { method: 'GET', path: '/payment/invoices' },
      list_customer_package_invoices: { method: 'GET', path: '/payment/invoices/customer-package' },
      verify_customer_package_payment: {
        method: 'POST',
        path: '/payment/invoices/:id/customer-package/verify',
      },
      verify_payment: {
        method: 'POST',
        path: '/payment/invoices/:invoiceId/verify',
      },
      list_all_invoices: { method: 'GET', path: '/payment/invoices/all' },
      get_fx_rate: { method: 'GET', path: '/payment/fx-rate' },
      pay_invoice_midtrans: { method: 'POST', path: '/payment/invoices/:id/midtrans' },
      check_payment_status: { method: 'GET', path: '/payment/invoices/:id/status' },
      submit_payment_proof: { method: 'POST', path: '/payment/invoices/:invoiceId/proof' },

      // Notifications
      list_notifications: { method: 'GET', path: '/notifications' },
      get_unread_count: { method: 'GET', path: '/notifications/unread-count' },
      mark_as_read: { method: 'POST', path: '/notifications/:id/read' },
      mark_all_as_read: { method: 'POST', path: '/notifications/read-all' },
      delete_notification: { method: 'DELETE', path: '/notifications/:id' },
      get_preferences: { method: 'GET', path: '/notifications/preferences' },
      update_preference: { method: 'PUT', path: '/notifications/preferences' },
      subscribe_push: { method: 'POST', path: '/notifications/push/subscribe' },
      unsubscribe_push: { method: 'POST', path: '/notifications/push/unsubscribe' },
      send_test_notification: { method: 'POST', path: '/notifications/test' },

      // Email Outbox (Admin)
      list_email_outbox: { method: 'GET', path: '/email-outbox' },
      get_email_outbox: { method: 'GET', path: '/email-outbox/:id' },
      get_email_outbox_stats: { method: 'GET', path: '/email-outbox/stats' },
      retry_email_outbox: { method: 'POST', path: '/email-outbox/:id/retry' },
      delete_email_outbox: { method: 'DELETE', path: '/email-outbox/:id' },
      bulk_retry_email_outbox: { method: 'POST', path: '/email-outbox/bulk/retry' },
      bulk_delete_email_outbox: { method: 'POST', path: '/email-outbox/bulk/delete' },
      export_email_outbox_csv: { method: 'GET', path: '/email-outbox/export' },

      // Announcements
      list_active_announcements: { method: 'GET', path: '/announcements/active' },
      list_recent_announcements: { method: 'GET', path: '/announcements/recent' },
      get_announcement: { method: 'GET', path: '/announcements/:id' },
      dismiss_announcement: { method: 'POST', path: '/announcements/:id/dismiss' },
      list_announcements_admin: { method: 'GET', path: '/announcements/admin' },
      create_announcement_admin: { method: 'POST', path: '/announcements/admin' },
      update_announcement_admin: { method: 'PUT', path: '/announcements/admin/:id' },
      delete_announcement_admin: { method: 'DELETE', path: '/announcements/admin/:id' },

      // Tenant
      get_current_tenant: { method: 'GET', path: '/tenant/me' },
      update_current_tenant: { method: 'PUT', path: '/tenant/me' },
      // Audit Logs (Admin / Tenant scoped)
      list_tenant_audit_logs: { method: 'GET', path: '/admin/audit-logs' },

      // MikroTik routers (Tenant admin)
      list_mikrotik_routers: { method: 'GET', path: '/admin/mikrotik/routers' },
      list_mikrotik_noc: { method: 'GET', path: '/admin/mikrotik/noc' },
      list_mikrotik_alerts: { method: 'GET', path: '/admin/mikrotik/alerts' },
      list_mikrotik_incidents: { method: 'GET', path: '/admin/mikrotik/incidents' },
      list_mikrotik_logs: { method: 'GET', path: '/admin/mikrotik/logs' },
      ack_mikrotik_alert: { method: 'POST', path: '/admin/mikrotik/alerts/:id/ack' },
      resolve_mikrotik_alert: { method: 'POST', path: '/admin/mikrotik/alerts/:id/resolve' },
      ack_mikrotik_incident: { method: 'POST', path: '/admin/mikrotik/incidents/:id/ack' },
      resolve_mikrotik_incident: { method: 'POST', path: '/admin/mikrotik/incidents/:id/resolve' },
      update_mikrotik_incident: { method: 'PUT', path: '/admin/mikrotik/incidents/:id' },
      simulate_mikrotik_incident: { method: 'POST', path: '/admin/mikrotik/incidents/simulate' },
      run_mikrotik_incident_auto_escalation: {
        method: 'POST',
        path: '/admin/mikrotik/incidents/escalate-now',
      },
      create_mikrotik_router: { method: 'POST', path: '/admin/mikrotik/routers' },
      update_mikrotik_router: { method: 'PUT', path: '/admin/mikrotik/routers/:id' },
      delete_mikrotik_router: { method: 'DELETE', path: '/admin/mikrotik/routers/:id' },
      test_mikrotik_router: { method: 'POST', path: '/admin/mikrotik/routers/:id/test' },
      get_mikrotik_router: { method: 'GET', path: '/admin/mikrotik/routers/:id' },
      get_mikrotik_router_snapshot: { method: 'GET', path: '/admin/mikrotik/routers/:id/snapshot' },
      list_mikrotik_router_metrics: {
        method: 'GET',
        path: '/admin/mikrotik/routers/:routerId/metrics',
      },
      list_mikrotik_interface_metrics: {
        method: 'GET',
        path: '/admin/mikrotik/routers/:routerId/interfaces/metrics',
      },
      list_mikrotik_interface_latest: {
        method: 'GET',
        path: '/admin/mikrotik/routers/:routerId/interfaces/latest',
      },
      get_mikrotik_live_interface_counters: {
        method: 'GET',
        path: '/admin/mikrotik/routers/:routerId/interfaces/live',
      },
      list_mikrotik_ppp_profiles: {
        method: 'GET',
        path: '/admin/mikrotik/routers/:routerId/ppp-profiles',
      },
      sync_mikrotik_ppp_profiles: {
        method: 'POST',
        path: '/admin/mikrotik/routers/:routerId/ppp-profiles/sync',
      },
      list_mikrotik_ip_pools: {
        method: 'GET',
        path: '/admin/mikrotik/routers/:routerId/ip-pools',
      },
      sync_mikrotik_ip_pools: {
        method: 'POST',
        path: '/admin/mikrotik/routers/:routerId/ip-pools/sync',
      },
      sync_mikrotik_logs: {
        method: 'POST',
        path: '/admin/mikrotik/routers/:routerId/logs/sync',
      },

      // PPPoE (Tenant admin)
      list_pppoe_accounts: { method: 'GET', path: '/admin/pppoe/accounts' },
      get_pppoe_account: { method: 'GET', path: '/admin/pppoe/accounts/:id' },
      create_pppoe_account: { method: 'POST', path: '/admin/pppoe/accounts' },
      update_pppoe_account: { method: 'PUT', path: '/admin/pppoe/accounts/:id' },
      delete_pppoe_account: { method: 'DELETE', path: '/admin/pppoe/accounts/:id' },
      apply_pppoe_account: { method: 'POST', path: '/admin/pppoe/accounts/:id/apply' },
      reconcile_pppoe_router: {
        method: 'POST',
        path: '/admin/pppoe/routers/:routerId/reconcile',
      },
      preview_pppoe_import_from_router: {
        method: 'GET',
        path: '/admin/pppoe/routers/:routerId/import/preview',
      },
      import_pppoe_from_router: {
        method: 'POST',
        path: '/admin/pppoe/routers/:routerId/import',
      },

      // ISP Packages (Tenant admin)
      list_isp_packages: { method: 'GET', path: '/admin/isp-packages/packages' },
      create_isp_package: { method: 'POST', path: '/admin/isp-packages/packages' },
      update_isp_package: { method: 'PUT', path: '/admin/isp-packages/packages/:id' },
      delete_isp_package: { method: 'DELETE', path: '/admin/isp-packages/packages/:id' },
      list_isp_package_router_mappings: {
        method: 'GET',
        path: '/admin/isp-packages/router-mappings',
      },
      upsert_isp_package_router_mapping: {
        method: 'POST',
        path: '/admin/isp-packages/router-mappings',
      },
      // Network Mapping (tenant scoped)
      list_network_nodes: { method: 'GET', path: '/admin/network-mapping/nodes' },
      create_network_node: { method: 'POST', path: '/admin/network-mapping/nodes' },
      update_network_node: { method: 'PATCH', path: '/admin/network-mapping/nodes/:id' },
      delete_network_node: { method: 'DELETE', path: '/admin/network-mapping/nodes/:id' },
      list_network_links: { method: 'GET', path: '/admin/network-mapping/links' },
      create_network_link: { method: 'POST', path: '/admin/network-mapping/links' },
      update_network_link: { method: 'PATCH', path: '/admin/network-mapping/links/:id' },
      delete_network_link: { method: 'DELETE', path: '/admin/network-mapping/links/:id' },
      list_service_zones: { method: 'GET', path: '/admin/network-mapping/zones' },
      create_service_zone: { method: 'POST', path: '/admin/network-mapping/zones' },
      update_service_zone: { method: 'PATCH', path: '/admin/network-mapping/zones/:id' },
      delete_service_zone: { method: 'DELETE', path: '/admin/network-mapping/zones/:id' },
      resolve_service_zone: { method: 'POST', path: '/admin/network-mapping/zones/resolve' },
      compute_network_path: { method: 'POST', path: '/admin/network-mapping/paths/compute' },
      check_network_coverage: { method: 'POST', path: '/admin/network-mapping/coverage/check' },
      list_zone_offers: { method: 'GET', path: '/admin/network-mapping/zone-offers' },
      create_zone_offer: { method: 'POST', path: '/admin/network-mapping/zone-offers' },
      update_zone_offer: { method: 'PATCH', path: '/admin/network-mapping/zone-offers/:id' },
      delete_zone_offer: { method: 'DELETE', path: '/admin/network-mapping/zone-offers/:id' },
      list_zone_node_bindings: {
        method: 'GET',
        path: '/admin/network-mapping/zone-node-bindings',
      },
      create_zone_node_binding: {
        method: 'POST',
        path: '/admin/network-mapping/zone-node-bindings',
      },
      delete_zone_node_binding: {
        method: 'DELETE',
        path: '/admin/network-mapping/zone-node-bindings/:id',
      },

      // Backup
      list_backups: { method: 'GET', path: '/backups' },
      create_backup: { method: 'POST', path: '/backups' },
      delete_backup: { method: 'DELETE', path: '/backups/:filename' },
    };

    let route = commandMap[command];
    if (route) {
      // Helper to convert camelCase to snake_case
      const toSnakeCase = (str: string) =>
        str.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);

      // Handle path parameters (e.g. :key, :id)
      let path = route.path;
      const queryParams: Record<string, string> = {};
      const consumedPathKeys = new Set<string>();

      const toCamelCase = (str: string) =>
        str.replace(/_([a-z])/g, (_m, c: string) => c.toUpperCase());

      if (args) {
        for (const [key, value] of Object.entries(args)) {
          if (key.startsWith('__')) continue;
          // Skip null/undefined values to avoid 'null' string in query params
          if (value === null || value === undefined) continue;

          // Try both camelCase and snake_case for path params
          const snakeKey = toSnakeCase(key);
          if (path.includes(`:${key}`)) {
            path = path.replace(`:${key}`, String(value));
            consumedPathKeys.add(key);
            consumedPathKeys.add(snakeKey);
            consumedPathKeys.add(toCamelCase(key));
          } else if (path.includes(`:${snakeKey}`)) {
            path = path.replace(`:${snakeKey}`, String(value));
            consumedPathKeys.add(key);
            consumedPathKeys.add(snakeKey);
            consumedPathKeys.add(toCamelCase(key));
          } else if (route.method === 'GET' && key !== 'token') {
            // Add non-path params as query params for GET requests (use snake_case for HTTP)
            queryParams[snakeKey] = String(value);
          }
        }
      }

      // Build query string for GET requests
      const queryString =
        Object.keys(queryParams).length > 0
          ? '?' + new URLSearchParams(queryParams).toString()
          : '';

      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
      };

      // Add token if available in args or storage
      const token =
        args?.token || localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }

      const controller = typeof AbortController !== 'undefined' ? new AbortController() : null;
      const timeoutMs =
        typeof args?.__timeout_ms === 'number' && Number.isFinite(args.__timeout_ms)
          ? Math.max(1, args.__timeout_ms)
          : 15000;
      const externalSignal: AbortSignal | undefined = args?.__signal;
      let abortedByExternalSignal = false;
      let abortedByTimeout = false;
      const onExternalAbort = () => {
        abortedByExternalSignal = true;
        controller?.abort();
      };

      if (externalSignal) {
        if (externalSignal.aborted) onExternalAbort();
        else externalSignal.addEventListener('abort', onExternalAbort, { once: true });
      }

      const timeout = setTimeout(() => {
        abortedByTimeout = true;
        controller?.abort();
      }, timeoutMs);

      let response: Response;
      try {
        // `token` must only be sent via Authorization header.
        // Some endpoints use strict payload validation (`deny_unknown_fields`)
        // and will reject unexpected fields like `token`.
        const bodyPayload =
          route.method !== 'GET'
            ? (() => {
                const rawEntries = Object.entries(args || {}).filter(
                  ([key, value]) =>
                    key !== 'token' &&
                    !key.startsWith('__') &&
                    value !== undefined &&
                    !consumedPathKeys.has(key),
                );

                // If both camelCase and snake_case variants exist, keep camelCase only.
                const keySet = new Set(rawEntries.map(([key]) => key));
                const deduped = rawEntries.filter(([key]) => {
                  if (!key.includes('_')) return true;
                  const camelKey = toCamelCase(key);
                  return !keySet.has(camelKey);
                });

                return Object.fromEntries(deduped);
              })()
            : undefined;

        response = await fetch(`${API_BASE}${path}${queryString}`, {
          method: route.method,
          headers,
          body: bodyPayload ? JSON.stringify(bodyPayload) : undefined,
          signal: controller?.signal,
        });
      } catch (e: any) {
        const name = String(e?.name || '');
        if (name === 'AbortError') {
          if (abortedByExternalSignal) {
            throw new Error('Request canceled');
          }
          if (abortedByTimeout) {
            throw new Error('Remote API request timed out. Is the server running?');
          }
          throw new Error('Request aborted');
        }
        throw e;
      } finally {
        clearTimeout(timeout);
        if (externalSignal) externalSignal.removeEventListener('abort', onExternalAbort);
      }

      const contentType = response.headers.get('content-type') || '';
      const isJson = contentType.toLowerCase().includes('application/json');

      if (!response.ok) {
        // Only 401 should force auth-expired logout flow.
        // 403 is usually permission/role denial and must not clear session.
        if (response.status === 401) {
          handleAuthExpired(`HTTP ${response.status} from ${command}`);
        }
        if (isJson) {
          const errorBody = await response.json().catch(() => ({}));
          const message =
            errorBody?.error ||
            errorBody?.message ||
            errorBody?.detail ||
            errorBody?.details ||
            `HTTP Error ${response.status}`;
          throw new Error(message);
        }
        const errorText = (await response.text().catch(() => '')).trim();
        throw new Error(errorText || `HTTP Error ${response.status}`);
      }

      // Some endpoints (e.g. DELETE /storage/files/:id) return empty 200/204 body.
      if (response.status === 204) return undefined as T;

      const raw = await response.text();
      if (!raw || !raw.trim()) return undefined as T;
      if (isJson) return JSON.parse(raw) as T;

      // For non-JSON success payloads, return raw text.
      return raw as T;
    }

    console.warn(`[Mock] Calling ${command} with`, args);
    // Return mock data for unimplemented endpoints
    if (command === 'get_current_user') return null as any;
    if (command === 'get_all_settings') return [] as any;
    if (command === 'list_users') return { data: [], total: 0, page: 1, per_page: 10 } as any;
    if (command === 'validate_token') return true as any;
    if (command === 'is_installed') return false as any; // DEFAULT TO FALSE SO WE DON'T BYPASS INSTALL
    if (command === 'get_auth_settings')
      return {
        jwt_expiry_hours: 24,
        session_timeout_minutes: 60,
        password_min_length: 8,
        password_require_uppercase: true,
        password_require_number: true,
        password_require_special: false,
        max_login_attempts: 5,
        lockout_duration_minutes: 15,
        allow_registration: true,
      } as any;

    throw new Error(`Command '${command}' not implemented in HTTP API yet.`);
  } catch (error: any) {
    // Downgrade 401/Invalid token errors to warnings as they are handled by the app (logout)
    const isAuthError =
      error.message?.includes('401') ||
      error.message?.includes('Invalid token') ||
      error.message?.includes('Unauthorized');

    if (isAuthError) {
      handleAuthExpired(`Auth error from ${command}: ${error.message || error}`);
      console.warn(`API Warning (${command}):`, error.message);
    } else {
      console.error(`API Error (${command}):`, error);
    }
    throw error;
  }
}

// Helper to get token
function getTokenOrThrow(): string {
  if (typeof window === 'undefined') throw new Error('Client side only');
  const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
  if (!token) throw new Error('Authentication required');
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
  two_factor_enabled?: boolean;
  totp_enabled?: boolean;
  email_2fa_enabled?: boolean;
  created_at: string;
  permissions: string[];
  tenant_slug?: string;
  tenant_id?: string;
  tenant_role?: string;
  tenant_custom_domain?: string;
  preferred_2fa_method?: string;
}

export interface UserAddress {
  id: string;
  user_id: string;
  label?: string | null;
  recipient_name?: string | null;
  phone?: string | null;
  line1: string;
  line2?: string | null;
  city?: string | null;
  state?: string | null;
  postal_code?: string | null;
  country_code: string;
  is_default_shipping: boolean;
  is_default_billing: boolean;
  created_at: string;
  updated_at: string;
}

export interface TrustedDevice {
  id: string;
  user_id: string;
  device_fingerprint: string;
  ip_address?: string;
  user_agent?: string;
  trusted_at: string;
  expires_at: string;
  last_used_at?: string;
}

export interface Tenant {
  id: string;
  name: string;
  slug: string;
  custom_domain?: string;
  logo_url?: string;
  is_active: boolean;
  enforce_2fa: boolean;
  created_at: string;
  updated_at: string;
}

export interface AuthResponse {
  user: User;
  tenant?: Tenant; // Use explicit type
  token?: string;
  expires_at?: string;
  message?: string;
  requires_2fa?: boolean;
  temp_token?: string;
  available_2fa_methods?: string[];
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
  session_timeout_minutes: number;
  password_min_length: number;
  password_require_uppercase: boolean;
  password_require_number: boolean;
  password_require_special: boolean;
  max_login_attempts: number;
  lockout_duration_minutes: number;
  allow_registration: boolean;
  main_domain?: string;
}

export interface EmailVerificationReadiness {
  ready: boolean;
  reason?: string | null;
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

export interface SupportTicketListItem {
  id: string;
  tenant_id: string;
  created_by: string | null;
  created_by_name?: string | null;
  subject: string;
  status: 'open' | 'pending' | 'closed' | string;
  priority: 'low' | 'normal' | 'high' | 'urgent' | string;
  assigned_to: string | null;
  created_at: string;
  updated_at: string;
  closed_at: string | null;
  message_count: number;
  last_message_at: string | null;
}

export interface SupportTicketStats {
  all: number;
  open: number;
  pending: number;
  closed: number;
}

export interface SupportTicketMessage {
  id: string;
  ticket_id: string;
  author_id: string | null;
  body: string;
  is_internal: boolean;
  created_at: string;
  attachments: FileRecord[];
}

export interface SupportTicket {
  id: string;
  tenant_id: string;
  created_by: string | null;
  subject: string;
  status: string;
  priority: string;
  assigned_to: string | null;
  created_at: string;
  updated_at: string;
  closed_at: string | null;
}

export interface SupportTicketDetail {
  ticket: SupportTicket;
  messages: SupportTicketMessage[];
}

export interface Customer {
  id: string;
  tenant_id: string;
  name: string;
  email: string | null;
  phone: string | null;
  notes: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CustomerLocation {
  id: string;
  tenant_id: string;
  customer_id: string;
  label: string;
  address_line1: string | null;
  address_line2: string | null;
  city: string | null;
  state: string | null;
  postal_code: string | null;
  country: string | null;
  latitude: number | null;
  longitude: number | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface CustomerPortalUser {
  customer_user_id: string;
  user_id: string;
  email: string;
  name: string;
  created_at: string;
}

export interface CustomerRegistrationInviteView {
  id: string;
  tenant_id: string;
  created_by: string | null;
  max_uses: number;
  used_count: number;
  expires_at: string;
  is_revoked: boolean;
  revoked_at: string | null;
  last_used_at: string | null;
  note: string | null;
  created_at: string;
}

export interface CustomerRegistrationInviteCreateResponse {
  invite: CustomerRegistrationInviteView;
  invite_token: string;
  invite_url: string;
}

export interface CustomerRegistrationInvitePolicy {
  default_expires_in_hours: number;
  default_max_uses: number;
}

export interface CustomerRegistrationInviteSummary {
  total: number;
  active: number;
  revoked: number;
  expired: number;
  used_up: number;
  total_uses: number;
  total_capacity: number;
  utilization_percent: number;
  created_last_30d: number;
  used_last_30d: number;
}

export interface CustomerRegistrationInviteValidation {
  valid: boolean;
  status: string;
  message: string;
  expires_at: string | null;
  max_uses: number | null;
  used_count: number | null;
  remaining_uses: number | null;
}

export interface CustomerSubscription {
  id: string;
  tenant_id: string;
  customer_id: string;
  location_id: string;
  package_id: string;
  router_id: string | null;
  billing_cycle: 'monthly' | 'yearly' | string;
  price: number;
  currency_code: string;
  status: 'active' | 'pending_installation' | 'suspended' | 'cancelled' | string;
  starts_at: string | null;
  ends_at: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface CustomerSubscriptionView extends CustomerSubscription {
  package_name: string | null;
  location_label: string | null;
  router_name: string | null;
}

export interface CustomerPortalCheckoutResponse {
  subscription: CustomerSubscription;
  invoice: Invoice;
}

export interface InstallationWorkOrderView {
  id: string;
  tenant_id: string;
  subscription_id: string;
  invoice_id: string | null;
  customer_id: string;
  location_id: string;
  router_id: string | null;
  status: 'pending' | 'in_progress' | 'completed' | 'cancelled' | string;
  assigned_to: string | null;
  scheduled_at: string | null;
  completed_at: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
  customer_name: string | null;
  location_label: string | null;
  package_name: string | null;
  router_name: string | null;
  assigned_to_name: string | null;
  assigned_to_email: string | null;
}

export interface PppoeAccountPublic {
  id: string;
  tenant_id: string;
  router_id: string;
  customer_id: string;
  location_id: string;
  username: string;
  package_id: string | null;
  profile_id: string | null;
  router_profile_name: string | null;
  remote_address: string | null;
  address_pool: string | null;
  disabled: boolean;
  comment: string | null;
  router_present: boolean;
  router_secret_id: string | null;
  last_sync_at: string | null;
  last_error: string | null;
  created_at: string;
  updated_at: string;
}

export interface IspPackage {
  id: string;
  tenant_id: string;
  name: string;
  description: string | null;
  features: string[];
  is_active: boolean;
  price_monthly: number;
  price_yearly: number;
  created_at: string;
  updated_at: string;
}

export interface IspPackageRouterMappingView {
  id: string;
  tenant_id: string;
  router_id: string;
  package_id: string;
  package_name: string;
  router_profile_name: string;
  address_pool: string | null;
  created_at: string;
  updated_at: string;
}

export interface Announcement {
  id: string;
  tenant_id: string | null;
  created_by: string | null;
  cover_file_id?: string | null;
  title: string;
  body: string;
  severity: string;
  audience: string;
  mode: 'post' | 'banner';
  format: 'plain' | 'markdown' | 'html';
  deliver_in_app: boolean;
  deliver_email: boolean;
  deliver_email_force?: boolean;
  starts_at: string;
  ends_at: string | null;
  notified_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateAnnouncementDto {
  scope?: 'tenant' | 'global';
  tenant_id?: string | null;
  cover_file_id?: string | null;
  title: string;
  body: string;
  severity?: 'info' | 'success' | 'warning' | 'error';
  audience?: 'all' | 'admins';
  mode?: 'post' | 'banner';
  format?: 'plain' | 'markdown' | 'html';
  deliver_in_app?: boolean;
  deliver_email?: boolean;
  deliver_email_force?: boolean;
  starts_at?: string | null;
  ends_at?: string | null;
}

export interface UpdateAnnouncementDto {
  cover_file_id?: string | null;
  title?: string;
  body?: string;
  severity?: 'info' | 'success' | 'warning' | 'error';
  audience?: 'all' | 'admins';
  mode?: 'post' | 'banner';
  format?: 'plain' | 'markdown' | 'html';
  deliver_in_app?: boolean;
  deliver_email?: boolean;
  deliver_email_force?: boolean;
  starts_at?: string | null;
  ends_at?: string | null;
}

// Auth API
export const auth = {
  register: (email: string, password: string, name: string): Promise<AuthResponse> =>
    safeInvoke('register', { email, password, name }),

  login: (email: string, password: string): Promise<AuthResponse> =>
    safeInvoke('login', { email, password }),

  logout: (token: string): Promise<void> => safeInvoke('logout', { token }),

  changePassword: (token: string, oldPassword: string, newPassword: string): Promise<void> =>
    safeInvoke('change_password', { token, old_password: oldPassword, new_password: newPassword }),

  getCurrentUser: (token: string): Promise<User> => safeInvoke('get_current_user', { token }),

  validateToken: (token: string): Promise<boolean> => safeInvoke('validate_token', { token }),

  verifyEmail: (token: string): Promise<AuthResponse> => safeInvoke('verify_email', { token }),

  forgotPassword: (email: string): Promise<void> => safeInvoke('forgot_password', { email }),

  resetPassword: (token: string, password: string): Promise<void> =>
    safeInvoke('reset_password', { token, password }),

  enable2FA: (): Promise<{ secret: string; qr: string }> =>
    safeInvoke('enable_2fa', { token: getTokenOrThrow() }),

  verify2FASetup: (secret: string, code: string): Promise<{ recovery_codes: string[] }> =>
    safeInvoke('verify_2fa_setup', { token: getTokenOrThrow(), secret, code }),

  disable2FA: (code: string): Promise<void> =>
    safeInvoke('disable_2fa', { token: getTokenOrThrow(), code }),

  request2FADisableCode: (): Promise<void> =>
    safeInvoke('request_2fa_disable_code', { token: getTokenOrThrow() }),

  resetUser2FA: (userId: string): Promise<void> =>
    safeInvoke('reset_user_2fa', { token: getTokenOrThrow(), userId }),

  verifyLogin2FA: (tempToken: string, code: string, trustDevice?: boolean): Promise<AuthResponse> =>
    safeInvoke('verify_login_2fa', { tempToken: tempToken, code, trustDevice: trustDevice }),

  requestEmailOtp: (tempToken: string): Promise<void> =>
    safeInvoke('request_email_otp', { tempToken: tempToken }),

  verifyEmailOtp: (tempToken: string, code: string, trustDevice?: boolean): Promise<AuthResponse> =>
    safeInvoke('verify_email_otp', { tempToken: tempToken, code, trustDevice: trustDevice }),

  get2FAMethods: (): Promise<string[]> => safeInvoke('get_2fa_methods', {}),

  set2FAPreference: (method: string): Promise<void> =>
    safeInvoke('set_2fa_preference', { token: getTokenOrThrow(), method }),

  requestEmail2FASetup: (): Promise<void> =>
    safeInvoke('request_email_2fa_setup', { token: getTokenOrThrow() }),

  verifyEmail2FASetup: (code: string): Promise<void> =>
    safeInvoke('verify_email_2fa_setup', { token: getTokenOrThrow(), code }),

  listTrustedDevices: (): Promise<TrustedDevice[]> =>
    safeInvoke('list_trusted_devices', { token: getTokenOrThrow() }),

  revokeTrustedDevice: (deviceId: string): Promise<void> =>
    safeInvoke('revoke_trusted_device', { token: getTokenOrThrow(), deviceId }),
};

// Users API
export const users = {
  list: (page?: number, perPage?: number): Promise<PaginatedResponse<User>> =>
    safeInvoke('list_users', { token: getTokenOrThrow(), page, perPage }),

  get: (id: string): Promise<User> => safeInvoke('get_user', { token: getTokenOrThrow(), id }),

  create: (email: string, password: string, name: string): Promise<User> =>
    safeInvoke('create_user', { token: getTokenOrThrow(), email, password, name }),

  update: (
    id: string,
    data: {
      email?: string;
      name?: string;
      role?: string;
      isActive?: boolean;
    },
  ): Promise<User> =>
    safeInvoke('update_user', {
      token: getTokenOrThrow(),
      id,
      email: data.email,
      name: data.name,
      role: data.role,
      // HTTP handler expects camelCase `isActive`, Tauri command expects `is_active`.
      // Send both to keep both runtimes working.
      isActive: data.isActive,
      is_active: data.isActive,
    }),

  delete: (id: string): Promise<void> =>
    safeInvoke('delete_user', { token: getTokenOrThrow(), id }),

  // --- Addresses (Self) ---
  listMyAddresses: (): Promise<UserAddress[]> =>
    safeInvoke('list_my_addresses', { token: getTokenOrThrow() }),

  createMyAddress: (data: {
    label?: string;
    recipientName?: string;
    phone?: string;
    line1: string;
    line2?: string;
    city?: string;
    state?: string;
    postalCode?: string;
    countryCode?: string;
    isDefaultShipping?: boolean;
    isDefaultBilling?: boolean;
  }): Promise<UserAddress> =>
    safeInvoke('create_my_address', {
      token: getTokenOrThrow(),
      // send both camelCase and snake_case to work in HTTP + Tauri
      label: data.label,
      recipientName: data.recipientName,
      recipient_name: data.recipientName,
      phone: data.phone,
      line1: data.line1,
      line2: data.line2,
      city: data.city,
      state: data.state,
      postalCode: data.postalCode,
      postal_code: data.postalCode,
      countryCode: data.countryCode,
      country_code: data.countryCode,
      isDefaultShipping: data.isDefaultShipping,
      is_default_shipping: data.isDefaultShipping,
      isDefaultBilling: data.isDefaultBilling,
      is_default_billing: data.isDefaultBilling,
    }),

  updateMyAddress: (
    addressId: string,
    data: {
      label?: string;
      recipientName?: string;
      phone?: string;
      line1?: string;
      line2?: string;
      city?: string;
      state?: string;
      postalCode?: string;
      countryCode?: string;
      isDefaultShipping?: boolean;
      isDefaultBilling?: boolean;
    },
  ): Promise<UserAddress> =>
    safeInvoke('update_my_address', {
      token: getTokenOrThrow(),
      addressId,
      address_id: addressId,
      label: data.label,
      recipientName: data.recipientName,
      recipient_name: data.recipientName,
      phone: data.phone,
      line1: data.line1,
      line2: data.line2,
      city: data.city,
      state: data.state,
      postalCode: data.postalCode,
      postal_code: data.postalCode,
      countryCode: data.countryCode,
      country_code: data.countryCode,
      isDefaultShipping: data.isDefaultShipping,
      is_default_shipping: data.isDefaultShipping,
      isDefaultBilling: data.isDefaultBilling,
      is_default_billing: data.isDefaultBilling,
    }),

  deleteMyAddress: (addressId: string): Promise<void> =>
    safeInvoke('delete_my_address', {
      token: getTokenOrThrow(),
      addressId,
      address_id: addressId,
    }),
};

// Roles API
export const roles = {
  list: (): Promise<Role[]> => safeInvoke('get_roles', { token: getTokenOrThrow() }),

  getPermissions: (): Promise<Permission[]> =>
    safeInvoke('get_permissions', { token: getTokenOrThrow() }),

  get: (id: string): Promise<Role | null> =>
    safeInvoke('get_role', { token: getTokenOrThrow(), id, roleId: id }),

  create: (
    name: string,
    description: string | undefined,
    level: number,
    permissions: string[],
  ): Promise<Role> =>
    safeInvoke('create_new_role', {
      token: getTokenOrThrow(),
      name,
      description,
      level,
      permissions,
    }),

  update: (
    id: string,
    name?: string,
    description?: string,
    level?: number,
    permissions?: string[],
  ): Promise<Role> =>
    safeInvoke('update_existing_role', {
      token: getTokenOrThrow(),
      id,
      roleId: id,
      name,
      description,
      level,
      permissions,
    }),

  delete: (id: string): Promise<boolean> =>
    safeInvoke('delete_existing_role', { token: getTokenOrThrow(), id, roleId: id }),
};

// Team API
export const team = {
  list: (): Promise<TeamMember[]> => safeInvoke('list_team_members', { token: getTokenOrThrow() }),

  add: (email: string, name: string, roleId: string, password?: string): Promise<TeamMember> =>
    safeInvoke('add_team_member', { token: getTokenOrThrow(), email, name, roleId, password }),

  updateRole: (memberId: string, roleId: string): Promise<void> =>
    safeInvoke('update_team_member_role', {
      token: getTokenOrThrow(),
      id: memberId,
      memberId,
      roleId,
    }),

  remove: (memberId: string): Promise<void> =>
    safeInvoke('remove_team_member', { token: getTokenOrThrow(), id: memberId, memberId }),
};

// Super Admin API
export const superadmin = {
  listTenants: (): Promise<{ data: any[]; total: number }> =>
    safeInvoke('list_tenants', { token: getTokenOrThrow() }),

  createTenant: (
    name: string,
    slug: string,
    customDomain: string | null,
    ownerEmail: string,
    ownerPassword: string,
    planId?: string,
  ): Promise<any> =>
    safeInvoke('create_tenant', {
      token: getTokenOrThrow(),
      name,
      slug,
      customDomain,
      ownerEmail,
      ownerPassword,
      planId,
    }),

  deleteTenant: (id: string): Promise<void> =>
    safeInvoke('delete_tenant', { token: getTokenOrThrow(), id }),

  updateTenant: (
    id: string,
    name: string,
    slug: string,
    customDomain: string | null,
    isActive: boolean,
  ): Promise<any> =>
    safeInvoke('update_tenant', {
      token: getTokenOrThrow(),
      id,
      name,
      slug,
      customDomain,
      isActive,
    }),

  listAuditLogs: (
    page?: number,
    perPage?: number,
    filters?: {
      user_id?: string;
      tenant_id?: string;
      customer_id?: string;
      resource?: string;
      resource_id?: string;
      action?: string;
      date_from?: string;
      date_to?: string;
      search?: string;
    },
  ): Promise<PaginatedResponse<AuditLog>> =>
    safeInvoke('list_audit_logs', { token: getTokenOrThrow(), page, perPage, ...filters }),

  getSystemHealth: (): Promise<any> =>
    safeInvoke('get_system_health', { token: getTokenOrThrow() }),

  getSystemDiagnostics: (): Promise<any> =>
    safeInvoke('get_system_diagnostics', { token: getTokenOrThrow() }),
};

export const audit = {
  listTenant: (
    page?: number,
    perPage?: number,
    filters?: {
      user_id?: string;
      customer_id?: string;
      resource?: string;
      resource_id?: string;
      action?: string;
      date_from?: string;
      date_to?: string;
      search?: string;
    },
  ): Promise<PaginatedResponse<AuditLog>> =>
    safeInvoke('list_tenant_audit_logs', { token: getTokenOrThrow(), page, perPage, ...filters }),
};

export const mikrotik = {
  routers: {
    noc: (): Promise<any[]> => safeInvoke('list_mikrotik_noc', { token: getTokenOrThrow() }),
    list: (): Promise<any[]> => safeInvoke('list_mikrotik_routers', { token: getTokenOrThrow() }),
    get: (id: string): Promise<any> =>
      safeInvoke('get_mikrotik_router', { token: getTokenOrThrow(), id }),
    snapshot: (id: string): Promise<any> =>
      safeInvoke('get_mikrotik_router_snapshot', { token: getTokenOrThrow(), id }),
    create: (router: {
      name: string;
      host: string;
      port?: number;
      username: string;
      password: string;
      use_tls?: boolean;
      enabled?: boolean;
      maintenance_until?: string | null;
      maintenance_reason?: string | null;
      latitude?: number | null;
      longitude?: number | null;
    }): Promise<any> =>
      safeInvoke('create_mikrotik_router', {
        token: getTokenOrThrow(),
        name: router.name,
        host: router.host,
        port: router.port,
        username: router.username,
        password: router.password,
        use_tls: router.use_tls,
        useTls: router.use_tls,
        enabled: router.enabled,
        maintenance_until: router.maintenance_until,
        maintenanceUntil: router.maintenance_until,
        maintenance_reason: router.maintenance_reason,
        maintenanceReason: router.maintenance_reason,
        latitude: router.latitude,
        longitude: router.longitude,
      }),
    update: (
      id: string,
      router: {
        name?: string;
        host?: string;
        port?: number;
        username?: string;
        password?: string;
        use_tls?: boolean;
        enabled?: boolean;
        maintenance_until?: string | null;
        maintenance_reason?: string | null;
        latitude?: number | null;
        longitude?: number | null;
      },
    ): Promise<any> =>
      safeInvoke('update_mikrotik_router', {
        token: getTokenOrThrow(),
        id,
        name: router.name,
        host: router.host,
        port: router.port,
        username: router.username,
        password: router.password,
        use_tls: router.use_tls,
        useTls: router.use_tls,
        enabled: router.enabled,
        // Always pass maintenance fields so server can clear them.
        maintenance_until: router.maintenance_until ?? null,
        maintenanceUntil: router.maintenance_until ?? null,
        maintenance_reason: router.maintenance_reason ?? null,
        maintenanceReason: router.maintenance_reason ?? null,
        latitude: router.latitude ?? null,
        longitude: router.longitude ?? null,
      }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_mikrotik_router', { token: getTokenOrThrow(), id }),
    test: (id: string): Promise<any> =>
      safeInvoke('test_mikrotik_router', { token: getTokenOrThrow(), id }),
    metrics: (routerId: string, limit?: number): Promise<any[]> =>
      safeInvoke('list_mikrotik_router_metrics', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
        limit,
      }),
    interfaceMetrics: (
      routerId: string,
      params?: { interface?: string; limit?: number },
    ): Promise<any[]> =>
      safeInvoke('list_mikrotik_interface_metrics', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
        interface: params?.interface,
        limit: params?.limit,
      }),
    interfaceLatest: (routerId: string): Promise<any[]> =>
      safeInvoke('list_mikrotik_interface_latest', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
    interfaceLive: (routerId: string, names: string[]): Promise<any[]> =>
      safeInvoke('get_mikrotik_live_interface_counters', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
        names,
      }),
    pppProfiles: (routerId: string): Promise<any[]> =>
      safeInvoke('list_mikrotik_ppp_profiles', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
    syncPppProfiles: (routerId: string): Promise<any[]> =>
      safeInvoke('sync_mikrotik_ppp_profiles', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
    ipPools: (routerId: string): Promise<any[]> =>
      safeInvoke('list_mikrotik_ip_pools', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
    syncIpPools: (routerId: string): Promise<any[]> =>
      safeInvoke('sync_mikrotik_ip_pools', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
  },
  alerts: {
    list: (params?: { activeOnly?: boolean; limit?: number }): Promise<any[]> =>
      safeInvoke('list_mikrotik_alerts', {
        token: getTokenOrThrow(),
        active_only: params?.activeOnly,
        activeOnly: params?.activeOnly,
        limit: params?.limit,
      }),
    ack: (id: string): Promise<any> =>
      safeInvoke('ack_mikrotik_alert', { token: getTokenOrThrow(), id }),
    resolve: (id: string): Promise<any> =>
      safeInvoke('resolve_mikrotik_alert', { token: getTokenOrThrow(), id }),
  },
  incidents: {
    list: (params?: { activeOnly?: boolean; limit?: number }): Promise<any[]> =>
      safeInvoke('list_mikrotik_incidents', {
        token: getTokenOrThrow(),
        active_only: params?.activeOnly,
        activeOnly: params?.activeOnly,
        limit: params?.limit,
      }),
    ack: (id: string): Promise<any> =>
      safeInvoke('ack_mikrotik_incident', { token: getTokenOrThrow(), id }),
    resolve: (id: string): Promise<any> =>
      safeInvoke('resolve_mikrotik_incident', { token: getTokenOrThrow(), id }),
    update: (
      id: string,
      payload: { ownerUserId?: string | null; notes?: string | null },
    ): Promise<any> =>
      safeInvoke('update_mikrotik_incident', {
        token: getTokenOrThrow(),
        id,
        owner_user_id: payload.ownerUserId ?? null,
        ownerUserId: payload.ownerUserId ?? null,
        notes: payload.notes ?? null,
      }),
    simulate: (payload: {
      routerId: string;
      incidentType: string;
      severity?: string;
      interfaceName?: string | null;
      message?: string | null;
    }): Promise<any> =>
      safeInvoke('simulate_mikrotik_incident', {
        token: getTokenOrThrow(),
        router_id: payload.routerId,
        routerId: payload.routerId,
        incident_type: payload.incidentType,
        incidentType: payload.incidentType,
        severity: payload.severity,
        interface_name: payload.interfaceName ?? null,
        interfaceName: payload.interfaceName ?? null,
        message: payload.message ?? null,
      }),
    runAutoEscalation: (): Promise<{ ok: boolean; escalated: number }> =>
      safeInvoke('run_mikrotik_incident_auto_escalation', { token: getTokenOrThrow() }),
  },
  logs: {
    list: (params?: {
      routerId?: string;
      level?: string;
      topic?: string;
      q?: string;
      page?: number;
      perPage?: number;
      includeTotal?: boolean;
    }): Promise<PaginatedResponse<any>> =>
      safeInvoke('list_mikrotik_logs', {
        token: getTokenOrThrow(),
        router_id: params?.routerId,
        routerId: params?.routerId,
        level: params?.level,
        topic: params?.topic,
        q: params?.q,
        page: params?.page,
        per_page: params?.perPage,
        include_total: params?.includeTotal,
        includeTotal: params?.includeTotal,
      }),
    sync: (routerId: string, fetchLimit?: number): Promise<any> =>
      safeInvoke('sync_mikrotik_logs', {
        token: getTokenOrThrow(),
        router_id: routerId,
        routerId,
        fetch_limit: fetchLimit,
        fetchLimit,
      }),
  },
};

export const support = {
  list: (params?: {
    status?: string;
    search?: string;
    page?: number;
    perPage?: number;
  }): Promise<PaginatedResponse<SupportTicketListItem>> =>
    safeInvoke('list_support_tickets', {
      token: getTokenOrThrow(),
      status: params?.status,
      search: params?.search,
      page: params?.page,
      per_page: params?.perPage,
    }),

  stats: (): Promise<SupportTicketStats> =>
    safeInvoke('get_support_ticket_stats', { token: getTokenOrThrow() }),

  create: (
    subject: string,
    message: string,
    priority?: string,
    attachmentIds?: string[],
  ): Promise<SupportTicketDetail> =>
    safeInvoke('create_support_ticket', {
      token: getTokenOrThrow(),
      subject,
      message,
      priority,
      attachmentIds,
      attachment_ids: attachmentIds,
    }),

  get: (id: string): Promise<SupportTicketDetail> =>
    safeInvoke('get_support_ticket', { token: getTokenOrThrow(), id }),

  reply: (
    id: string,
    message: string,
    isInternal?: boolean,
    attachmentIds?: string[],
  ): Promise<SupportTicketMessage> =>
    safeInvoke('reply_support_ticket', {
      token: getTokenOrThrow(),
      id,
      message,
      isInternal,
      is_internal: isInternal,
      attachmentIds,
      attachment_ids: attachmentIds,
    }),

  update: (
    id: string,
    data: { status?: string; priority?: string; assignedTo?: string | null },
  ): Promise<SupportTicket> =>
    safeInvoke('update_support_ticket', {
      token: getTokenOrThrow(),
      id,
      status: data.status,
      priority: data.priority,
      assignedTo: data.assignedTo ?? undefined,
      assigned_to: data.assignedTo ?? undefined,
    }),
};

export const customers = {
  list: (params?: {
    q?: string;
    page?: number;
    perPage?: number;
  }): Promise<PaginatedResponse<Customer>> =>
    safeInvoke('list_customers', {
      token: getTokenOrThrow(),
      q: params?.q,
      page: params?.page,
      per_page: params?.perPage,
    }),

  get: (customerId: string): Promise<Customer> =>
    safeInvoke('get_customer', {
      token: getTokenOrThrow(),
      customerId,
      customer_id: customerId,
    }),

  create: (dto: {
    name: string;
    email?: string | null;
    phone?: string | null;
    notes?: string | null;
    is_active?: boolean;
  }): Promise<Customer> =>
    safeInvoke('create_customer', {
      token: getTokenOrThrow(),
      ...dto,
    }),

  createWithPortal: (dto: {
    name: string;
    email?: string | null;
    phone?: string | null;
    notes?: string | null;
    is_active?: boolean;
    portal_email: string;
    portal_name?: string | null;
    portal_password: string;
  }): Promise<Customer> =>
    safeInvoke('create_customer_with_portal', {
      token: getTokenOrThrow(),
      ...dto,
    }),

  update: (
    customerId: string,
    dto: {
      name?: string;
      email?: string | null;
      phone?: string | null;
      notes?: string | null;
      is_active?: boolean;
    },
  ): Promise<Customer> =>
    safeInvoke('update_customer', {
      token: getTokenOrThrow(),
      customerId,
      customer_id: customerId,
      ...dto,
    }),

  delete: (customerId: string): Promise<void> =>
    safeInvoke('delete_customer', {
      token: getTokenOrThrow(),
      customerId,
      customer_id: customerId,
    }),

  invites: {
    list: (params?: {
      include_inactive?: boolean;
      limit?: number;
    }): Promise<CustomerRegistrationInviteView[]> =>
      safeInvoke('list_customer_registration_invites', {
        token: getTokenOrThrow(),
        include_inactive: params?.include_inactive,
        includeInactive: params?.include_inactive,
        limit: params?.limit,
      }),
    create: (dto?: {
      expires_in_hours?: number;
      max_uses?: number;
      note?: string | null;
    }): Promise<CustomerRegistrationInviteCreateResponse> =>
      safeInvoke('create_customer_registration_invite', {
        token: getTokenOrThrow(),
        expires_in_hours: dto?.expires_in_hours,
        max_uses: dto?.max_uses,
        note: dto?.note ?? undefined,
      }),
    revoke: (inviteId: string): Promise<void> =>
      safeInvoke('revoke_customer_registration_invite', {
        token: getTokenOrThrow(),
        inviteId,
        invite_id: inviteId,
      }),
    getPolicy: (): Promise<CustomerRegistrationInvitePolicy> =>
      safeInvoke('get_customer_registration_invite_policy', {
        token: getTokenOrThrow(),
      }),
    updatePolicy: (dto: {
      default_expires_in_hours?: number;
      default_max_uses?: number;
    }): Promise<CustomerRegistrationInvitePolicy> =>
      safeInvoke('update_customer_registration_invite_policy', {
        token: getTokenOrThrow(),
        default_expires_in_hours: dto.default_expires_in_hours,
        default_max_uses: dto.default_max_uses,
      }),
    summary: (): Promise<CustomerRegistrationInviteSummary> =>
      safeInvoke('get_customer_registration_invite_summary', {
        token: getTokenOrThrow(),
      }),
  },

  locations: {
    list: (customerId: string): Promise<CustomerLocation[]> =>
      safeInvoke('list_customer_locations', {
        token: getTokenOrThrow(),
        customerId,
        customer_id: customerId,
      }),
    create: (dto: {
      customer_id: string;
      label: string;
      address_line1?: string | null;
      address_line2?: string | null;
      city?: string | null;
      state?: string | null;
      postal_code?: string | null;
      country?: string | null;
      latitude?: number | null;
      longitude?: number | null;
      notes?: string | null;
    }): Promise<CustomerLocation> =>
      safeInvoke('create_customer_location', {
        token: getTokenOrThrow(),
        ...dto,
      }),
    update: (
      locationId: string,
      dto: Partial<
        Pick<
          CustomerLocation,
          | 'label'
          | 'address_line1'
          | 'address_line2'
          | 'city'
          | 'state'
          | 'postal_code'
          | 'country'
          | 'latitude'
          | 'longitude'
          | 'notes'
        >
      >,
    ): Promise<CustomerLocation> =>
      safeInvoke('update_customer_location', {
        token: getTokenOrThrow(),
        locationId,
        location_id: locationId,
        ...dto,
      }),
    delete: (locationId: string): Promise<void> =>
      safeInvoke('delete_customer_location', {
        token: getTokenOrThrow(),
        locationId,
        location_id: locationId,
      }),
  },

  portalUsers: {
    list: (customerId: string): Promise<CustomerPortalUser[]> =>
      safeInvoke('list_customer_portal_users', {
        token: getTokenOrThrow(),
        customerId,
        customer_id: customerId,
      }),
    addExisting: (dto: { customer_id: string; user_id: string }): Promise<CustomerPortalUser> =>
      safeInvoke('add_customer_portal_user', {
        token: getTokenOrThrow(),
        ...dto,
      }),
    createNew: (dto: {
      customer_id: string;
      email: string;
      name: string;
      password: string;
    }): Promise<CustomerPortalUser> =>
      safeInvoke('create_customer_portal_user', {
        token: getTokenOrThrow(),
        ...dto,
      }),
    remove: (customerUserId: string): Promise<void> =>
      safeInvoke('remove_customer_portal_user', {
        token: getTokenOrThrow(),
        customerUserId,
        customer_user_id: customerUserId,
      }),
  },

  subscriptions: {
    list: (
      customerId: string,
      params?: { page?: number; per_page?: number },
    ): Promise<PaginatedResponse<CustomerSubscriptionView>> =>
      safeInvoke('list_customer_subscriptions', {
        token: getTokenOrThrow(),
        customerId,
        customer_id: customerId,
        page: params?.page,
        per_page: params?.per_page,
      }),
    create: (
      customerId: string,
      dto: {
        location_id: string;
        package_id: string;
        router_id?: string | null;
        billing_cycle: 'monthly' | 'yearly' | string;
        price: number;
        currency_code?: string | null;
        status?: 'active' | 'suspended' | 'cancelled' | string;
        starts_at?: string | null;
        ends_at?: string | null;
        notes?: string | null;
      },
    ): Promise<CustomerSubscription> =>
      safeInvoke('create_customer_subscription', {
        token: getTokenOrThrow(),
        customerId,
        customer_id: customerId,
        ...dto,
      }),
    update: (
      subscriptionId: string,
      dto: {
        location_id?: string;
        package_id?: string;
        router_id?: string | null;
        billing_cycle?: 'monthly' | 'yearly' | string;
        price?: number;
        currency_code?: string | null;
        status?: 'active' | 'suspended' | 'cancelled' | string;
        starts_at?: string | null;
        ends_at?: string | null;
        notes?: string | null;
      },
    ): Promise<CustomerSubscription> =>
      safeInvoke('update_customer_subscription', {
        token: getTokenOrThrow(),
        subscriptionId,
        subscription_id: subscriptionId,
        ...dto,
      }),
    delete: (subscriptionId: string): Promise<void> =>
      safeInvoke('delete_customer_subscription', {
        token: getTokenOrThrow(),
        subscriptionId,
        subscription_id: subscriptionId,
      }),
  },

  portal: {
    myLocations: (): Promise<CustomerLocation[]> =>
      safeInvoke('list_my_customer_locations', { token: getTokenOrThrow() }),
    createMyLocation: (dto: {
      label: string;
      address_line1?: string | null;
      address_line2?: string | null;
      city?: string | null;
      state?: string | null;
      postal_code?: string | null;
      country?: string | null;
      latitude?: number | null;
      longitude?: number | null;
      notes?: string | null;
    }): Promise<CustomerLocation> =>
      safeInvoke('create_my_customer_location', { token: getTokenOrThrow(), ...dto }),
    myPackages: (): Promise<IspPackage[]> =>
      safeInvoke('list_my_customer_packages', { token: getTokenOrThrow() }),
    mySubscriptions: (params?: {
      page?: number;
      per_page?: number;
    }): Promise<PaginatedResponse<CustomerSubscriptionView>> =>
      safeInvoke('list_my_customer_subscriptions', {
        token: getTokenOrThrow(),
        page: params?.page,
        per_page: params?.per_page,
      }),
    checkout: (dto: {
      location_id: string;
      package_id: string;
      billing_cycle: 'monthly' | 'yearly' | string;
    }): Promise<CustomerPortalCheckoutResponse> =>
      safeInvoke('create_my_customer_subscription_invoice', {
        token: getTokenOrThrow(),
        ...dto,
      }),
  },
};

export const workOrders = {
  list: (params?: {
    status?: string;
    assigned_to?: string;
    include_closed?: boolean;
    limit?: number;
  }): Promise<InstallationWorkOrderView[]> =>
    safeInvoke('list_installation_work_orders', {
      token: getTokenOrThrow(),
      ...(params || {}),
    }),
  assign: (id: string, payload: { assigned_to: string; scheduled_at?: string; notes?: string }) =>
    safeInvoke('assign_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      ...payload,
    }),
  start: (id: string, notes?: string) =>
    safeInvoke('start_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      notes: notes ?? undefined,
    }),
  complete: (id: string, notes?: string) =>
    safeInvoke('complete_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      notes: notes ?? undefined,
    }),
  cancel: (id: string, notes?: string) =>
    safeInvoke('cancel_installation_work_order', {
      token: getTokenOrThrow(),
      id,
      notes: notes ?? undefined,
    }),
};

export const pppoe = {
  accounts: {
    list: (params?: {
      customer_id?: string;
      location_id?: string;
      router_id?: string;
      q?: string;
      page?: number;
      per_page?: number;
    }): Promise<PaginatedResponse<PppoeAccountPublic>> =>
      safeInvoke('list_pppoe_accounts', { token: getTokenOrThrow(), ...(params || {}) }),

    get: (id: string): Promise<PppoeAccountPublic> =>
      safeInvoke('get_pppoe_account', { token: getTokenOrThrow(), id }),

    create: (dto: {
      router_id: string;
      customer_id: string;
      location_id: string;
      username: string;
      password: string;
      package_id?: string | null;
      profile_id?: string | null;
      router_profile_name?: string | null;
      remote_address?: string | null;
      address_pool?: string | null;
      disabled?: boolean;
      comment?: string | null;
    }): Promise<PppoeAccountPublic> =>
      safeInvoke('create_pppoe_account', {
        token: getTokenOrThrow(),
        router_id: dto.router_id,
        customer_id: dto.customer_id,
        location_id: dto.location_id,
        username: dto.username,
        password: dto.password,
        package_id: dto.package_id ?? null,
        profile_id: dto.profile_id ?? null,
        router_profile_name: dto.router_profile_name ?? null,
        remote_address: dto.remote_address ?? null,
        address_pool: dto.address_pool ?? null,
        disabled: dto.disabled ?? false,
        comment: dto.comment ?? null,
      }),

    update: (
      id: string,
      dto: {
        username?: string;
        password?: string;
        package_id?: string | null;
        profile_id?: string | null;
        router_profile_name?: string | null;
        remote_address?: string | null;
        address_pool?: string | null;
        disabled?: boolean;
        comment?: string | null;
      },
    ): Promise<PppoeAccountPublic> =>
      safeInvoke('update_pppoe_account', {
        token: getTokenOrThrow(),
        id,
        username: dto.username,
        password: dto.password,
        package_id: dto.package_id ?? undefined,
        profile_id: dto.profile_id ?? undefined,
        router_profile_name: dto.router_profile_name ?? undefined,
        remote_address: dto.remote_address ?? undefined,
        address_pool: dto.address_pool ?? undefined,
        disabled: dto.disabled,
        comment: dto.comment ?? undefined,
      }),

    delete: (id: string): Promise<void> =>
      safeInvoke('delete_pppoe_account', { token: getTokenOrThrow(), id }),

    apply: (id: string): Promise<PppoeAccountPublic> =>
      safeInvoke('apply_pppoe_account', { token: getTokenOrThrow(), id }),

    reconcileRouter: (routerId: string): Promise<any> =>
      safeInvoke('reconcile_pppoe_router', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
      }),
  },
  import: {
    preview: (routerId: string, params?: { include_disabled?: boolean }): Promise<any[]> =>
      safeInvoke('preview_pppoe_import_from_router', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
        include_disabled: params?.include_disabled,
        includeDisabled: params?.include_disabled,
      }),
    run: (
      routerId: string,
      dto: { usernames: string[]; customer_id?: string; location_id?: string },
    ): Promise<any> =>
      safeInvoke('import_pppoe_from_router', {
        token: getTokenOrThrow(),
        routerId,
        router_id: routerId,
        usernames: dto.usernames,
        customer_id: dto.customer_id,
        location_id: dto.location_id,
      }),
  },
};

export const ispPackages = {
  packages: {
    list: (params?: {
      q?: string;
      page?: number;
      per_page?: number;
    }): Promise<PaginatedResponse<IspPackage>> =>
      safeInvoke('list_isp_packages', { token: getTokenOrThrow(), ...(params || {}) }),
    create: (dto: {
      name: string;
      description?: string | null;
      features?: string[];
      is_active?: boolean;
      price_monthly?: number;
      price_yearly?: number;
    }): Promise<IspPackage> =>
      safeInvoke('create_isp_package', {
        token: getTokenOrThrow(),
        name: dto.name,
        description: dto.description ?? null,
        features: dto.features ?? [],
        is_active: dto.is_active ?? true,
        price_monthly: dto.price_monthly ?? 0,
        price_yearly: dto.price_yearly ?? 0,
      }),
    update: (
      id: string,
      dto: {
        name?: string;
        description?: string | null;
        features?: string[];
        is_active?: boolean;
        price_monthly?: number;
        price_yearly?: number;
      },
    ): Promise<IspPackage> =>
      safeInvoke('update_isp_package', {
        token: getTokenOrThrow(),
        id,
        name: dto.name,
        description: dto.description ?? undefined,
        features: dto.features,
        is_active: dto.is_active,
        price_monthly: dto.price_monthly,
        price_yearly: dto.price_yearly,
      }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_isp_package', { token: getTokenOrThrow(), id }),
  },
  routerMappings: {
    list: (params?: { router_id?: string }): Promise<IspPackageRouterMappingView[]> =>
      safeInvoke('list_isp_package_router_mappings', {
        token: getTokenOrThrow(),
        router_id: params?.router_id,
      }),
    upsert: (dto: {
      router_id: string;
      package_id: string;
      router_profile_name: string;
      address_pool?: string | null;
    }): Promise<any> =>
      safeInvoke('upsert_isp_package_router_mapping', {
        token: getTokenOrThrow(),
        router_id: dto.router_id,
        package_id: dto.package_id,
        router_profile_name: dto.router_profile_name,
        address_pool: dto.address_pool ?? null,
      }),
  },
};

export const networkMapping = {
  nodes: {
    list: (params?: {
      q?: string;
      page?: number;
      per_page?: number;
      status?: string;
      kind?: string;
      bbox?: string;
    }, options?: { signal?: AbortSignal }): Promise<PaginatedResponse<any>> =>
      safeInvoke('list_network_nodes', {
        token: getTokenOrThrow(),
        ...(params || {}),
        __signal: options?.signal,
      }),
    create: (dto: any): Promise<any> =>
      safeInvoke('create_network_node', { token: getTokenOrThrow(), ...dto }),
    update: (id: string, dto: any): Promise<any> =>
      safeInvoke('update_network_node', { token: getTokenOrThrow(), id, ...dto }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_network_node', { token: getTokenOrThrow(), id }),
  },
  links: {
    list: (params?: {
      q?: string;
      page?: number;
      per_page?: number;
      status?: string;
      kind?: string;
      bbox?: string;
    }, options?: { signal?: AbortSignal }): Promise<PaginatedResponse<any>> =>
      safeInvoke('list_network_links', {
        token: getTokenOrThrow(),
        ...(params || {}),
        __signal: options?.signal,
      }),
    create: (dto: any): Promise<any> =>
      safeInvoke('create_network_link', { token: getTokenOrThrow(), ...dto }),
    update: (id: string, dto: any): Promise<any> =>
      safeInvoke('update_network_link', { token: getTokenOrThrow(), id, ...dto }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_network_link', { token: getTokenOrThrow(), id }),
  },
  paths: {
    compute: (dto: {
      source_node_id: string;
      target_node_id: string;
      max_hops?: number;
      max_utilization_pct?: number;
      allowed_link_types?: string[];
      allowed_statuses?: string[];
      exclude_link_ids?: string[];
      require_active_nodes?: boolean;
    }): Promise<any> =>
      safeInvoke('compute_network_path', { token: getTokenOrThrow(), ...dto }),
  },
  zones: {
    list: (params?: {
      q?: string;
      page?: number;
      per_page?: number;
      status?: string;
      kind?: string;
      bbox?: string;
    }, options?: { signal?: AbortSignal }): Promise<PaginatedResponse<any>> =>
      safeInvoke('list_service_zones', {
        token: getTokenOrThrow(),
        ...(params || {}),
        __signal: options?.signal,
      }),
    create: (dto: any): Promise<any> =>
      safeInvoke('create_service_zone', { token: getTokenOrThrow(), ...dto }),
    update: (id: string, dto: any): Promise<any> =>
      safeInvoke('update_service_zone', { token: getTokenOrThrow(), id, ...dto }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_service_zone', { token: getTokenOrThrow(), id }),
    resolve: (dto: { lat: number; lng: number }): Promise<any> =>
      safeInvoke('resolve_service_zone', { token: getTokenOrThrow(), ...dto }),
    checkCoverage: (dto: { lat: number; lng: number }): Promise<any> =>
      safeInvoke('check_network_coverage', { token: getTokenOrThrow(), ...dto }),
  },
  zoneOffers: {
    list: (params?: { zone_id?: string; package_id?: string; active_only?: boolean }): Promise<any[]> =>
      safeInvoke('list_zone_offers', { token: getTokenOrThrow(), ...(params || {}) }),
    create: (dto: any): Promise<any> =>
      safeInvoke('create_zone_offer', { token: getTokenOrThrow(), ...dto }),
    update: (id: string, dto: any): Promise<any> =>
      safeInvoke('update_zone_offer', { token: getTokenOrThrow(), id, ...dto }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_zone_offer', { token: getTokenOrThrow(), id }),
  },
  zoneBindings: {
    list: (params?: { zone_id?: string }): Promise<any[]> =>
      safeInvoke('list_zone_node_bindings', { token: getTokenOrThrow(), ...(params || {}) }),
    create: (dto: any): Promise<any> =>
      safeInvoke('create_zone_node_binding', { token: getTokenOrThrow(), ...dto }),
    delete: (id: string): Promise<void> =>
      safeInvoke('delete_zone_node_binding', { token: getTokenOrThrow(), id }),
  },
};

export const announcements = {
  listActive: (): Promise<Announcement[]> =>
    safeInvoke('list_active_announcements', { token: getTokenOrThrow() }),
  listRecent: (params?: {
    page?: number;
    per_page?: number;
    search?: string;
    severity?: string;
    mode?: string;
  }): Promise<PaginatedResponse<Announcement>> =>
    safeInvoke('list_recent_announcements', { token: getTokenOrThrow(), ...(params || {}) }),
  get: (id: string): Promise<Announcement> =>
    safeInvoke('get_announcement', { token: getTokenOrThrow(), id }),
  dismiss: (id: string): Promise<void> =>
    safeInvoke('dismiss_announcement', { token: getTokenOrThrow(), id }),

  listAdmin: (params?: {
    scope?: 'tenant' | 'global' | 'all';
    page?: number;
    per_page?: number;
    search?: string;
    severity?: string;
    mode?: string;
    status?: string;
  }): Promise<PaginatedResponse<Announcement>> =>
    safeInvoke('list_announcements_admin', { token: getTokenOrThrow(), ...(params || {}) }),
  createAdmin: (dto: CreateAnnouncementDto): Promise<Announcement> =>
    safeInvoke('create_announcement_admin', { token: getTokenOrThrow(), ...dto }),
  updateAdmin: (id: string, dto: UpdateAnnouncementDto): Promise<Announcement> =>
    safeInvoke('update_announcement_admin', { token: getTokenOrThrow(), id, ...dto }),
  deleteAdmin: (id: string): Promise<void> =>
    safeInvoke('delete_announcement_admin', { token: getTokenOrThrow(), id }),
};

// Public API (No Auth)
export const publicApi = {
  getTenant: (slug: string): Promise<any> => safeInvoke('get_tenant_by_slug', { slug }),
  getTenantByDomain: (domain: string): Promise<any> =>
    safeInvoke('get_tenant_by_domain', { domain }),
  getCustomerRegistrationStatusByDomain: (
    domain: string,
  ): Promise<{
    enabled: boolean;
    global_registration_enabled: boolean;
    tenant_self_registration_enabled: boolean;
  }> => safeInvoke('get_customer_registration_status_by_domain', { domain }),
  validateCustomerRegistrationInviteByDomain: (
    token: string,
  ): Promise<CustomerRegistrationInviteValidation> =>
    safeInvoke('validate_customer_registration_invite_by_domain', { token }),
  registerCustomerByDomain: (
    email: string,
    password: string,
    name: string,
    inviteToken?: string | null,
  ): Promise<AuthResponse> =>
    safeInvoke('register_customer_by_domain', {
      email,
      password,
      name,
      inviteToken: inviteToken ?? undefined,
      invite_token: inviteToken ?? undefined,
    }),
};

// Settings API
export const settings = {
  getAll: (): Promise<Setting[]> => safeInvoke('get_all_settings', { token: getTokenOrThrow() }),

  getPublicSettings: (): Promise<{
    app_name?: string;
    app_description?: string;
    default_locale?: string;
    app_timezone?: string;
    currency_code?: string;
    base_currency_code?: string;
    maintenance_mode?: boolean;
    maintenance_message?: string;
    payment_midtrans_enabled?: boolean;
    payment_midtrans_client_key?: string;
    payment_midtrans_is_production?: boolean;
    payment_manual_enabled?: boolean;
  }> => safeInvoke('get_public_settings'),

  getAuthSettings: (): Promise<AuthSettings> => safeInvoke('get_auth_settings'),

  get: (key: string): Promise<Setting | null> =>
    safeInvoke('get_setting', { token: getTokenOrThrow(), key }),

  getValue: (key: string): Promise<string | null> =>
    safeInvoke('get_setting_value', { token: getTokenOrThrow(), key }),

  getEmailVerificationReadiness: (): Promise<EmailVerificationReadiness> =>
    safeInvoke('get_email_verification_readiness', { token: getTokenOrThrow() }),

  upsert: (key: string, value: string, description?: string): Promise<Setting> =>
    safeInvoke('upsert_setting', { token: getTokenOrThrow(), key, value, description }),

  uploadLogo: (fileBase64: string): Promise<string> =>
    safeInvoke('upload_logo', { token: getTokenOrThrow(), content: fileBase64 }),

  getLogo: (token?: string): Promise<string | null> => safeInvoke('get_logo', { token }),

  delete: (key: string): Promise<void> =>
    safeInvoke('delete_setting', { token: getTokenOrThrow(), key }),

  sendTestEmail: (toEmail: string): Promise<string> =>
    safeInvoke('send_test_email', { token: getTokenOrThrow(), toEmail }),

  testSmtpConnection: (): Promise<SmtpConnectionTestResult> =>
    safeInvoke('test_smtp_connection', { token: getTokenOrThrow() }),

  getAppVersion: async (): Promise<string> => {
    const res = await safeInvoke('get_app_version');
    if (typeof res === 'object' && res !== null && 'version' in res) {
      return (res as any).version;
    }
    return (res as string) || '0.0.0';
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
    appUrl?: string,
  ): Promise<User> => {
    const res = await safeInvoke('install_app', {
      adminName,
      adminEmail,
      adminPassword,
      appName,
      appUrl,
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
  list: (): Promise<any[]> => safeInvoke('list_plans', { token: getTokenOrThrow() }),

  get: (planId: string): Promise<any> =>
    safeInvoke('get_plan', { token: getTokenOrThrow(), planId }),

  create: (
    name: string,
    slug: string,
    description?: string,
    price_monthly?: number,
    price_yearly?: number,
    is_active?: boolean,
    is_default?: boolean,
    sort_order?: number,
  ): Promise<any> =>
    safeInvoke('create_plan', {
      token: getTokenOrThrow(),
      name,
      slug,
      description,
      // NOTE: Tauri IPC arg mapping expects camelCase (it auto-converts to snake_case in Rust).
      // Keep both camelCase + snake_case to remain compatible with the HTTP API payloads.
      priceMonthly: price_monthly,
      priceYearly: price_yearly,
      isActive: is_active,
      isDefault: is_default,
      sortOrder: sort_order,
      price_monthly,
      price_yearly,
      is_active,
      is_default,
      sort_order,
    }),

  update: (
    planId: string,
    name?: string,
    slug?: string,
    description?: string,
    price_monthly?: number,
    price_yearly?: number,
    is_active?: boolean,
    is_default?: boolean,
    sort_order?: number,
  ): Promise<any> =>
    safeInvoke('update_plan', {
      token: getTokenOrThrow(),
      planId,
      name,
      slug,
      description,
      // NOTE: Tauri IPC arg mapping expects camelCase (it auto-converts to snake_case in Rust).
      // Keep both camelCase + snake_case to remain compatible with the HTTP API payloads.
      priceMonthly: price_monthly,
      priceYearly: price_yearly,
      isActive: is_active,
      isDefault: is_default,
      sortOrder: sort_order,
      price_monthly,
      price_yearly,
      is_active,
      is_default,
      sort_order,
    }),

  delete: (planId: string): Promise<void> =>
    safeInvoke('delete_plan', { token: getTokenOrThrow(), planId }),

  listFeatures: (): Promise<any[]> => safeInvoke('list_features', { token: getTokenOrThrow() }),

  // DEPRECATED: Feature creation is system managed
  createFeature: (
    code: string,
    name: string,
    description?: string,
    value_type?: string,
    category?: string,
    default_value?: string,
    sort_order?: number,
  ): Promise<any> =>
    safeInvoke('create_feature', {
      token: getTokenOrThrow(),
      code,
      name,
      description,
      value_type,
      category,
      default_value,
      sort_order,
    }),

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
  getSelf: (): Promise<any> => safeInvoke('get_current_tenant', { token: getTokenOrThrow() }),

  updateSelf: (data: {
    name?: string;
    customDomain?: string;
    enforce2fa?: boolean;
  }): Promise<any> =>
    safeInvoke('update_current_tenant', {
      token: getTokenOrThrow(),
      name: data.name,
      customDomain: data.customDomain,
      enforce2fa: data.enforce2fa,
    }),
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
  category: 'system' | 'team' | 'payment' | 'security' | 'support' | 'announcement' | string;
  action_url: string | null;
  is_read: boolean;
  created_at: string;
}

export interface BackupRecord {
  name: string;
  path: string;
  size: number;
  created_at: string;
  backup_type: string;
  tenant_id?: string;
}

export interface NotificationPreference {
  id: string;
  user_id: string;
  channel: 'in_app' | 'email' | 'push';
  category: 'system' | 'team' | 'payment' | 'security' | 'support' | 'announcement' | string;
  enabled: boolean;
  updated_at: string;
}

export interface EmailOutboxItem {
  id: string;
  tenant_id: string | null;
  to_email: string;
  subject: string;
  body: string;
  body_html: string | null;
  status: 'queued' | 'sending' | 'sent' | 'failed' | string;
  attempts: number;
  max_attempts: number;
  scheduled_at: string;
  last_error: string | null;
  sent_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface EmailOutboxStats {
  all: number;
  queued: number;
  sending: number;
  sent: number;
  failed: number;
}

export interface SmtpConnectionTestResult {
  ok: boolean;
  provider: string;
  host: string;
  port: number;
  encryption: string;
  duration_ms: number;
  message: string;
}

export interface Invoice {
  id: string;
  tenant_id?: string;
  invoice_number: string;
  amount: number;
  currency_code?: string;
  base_currency_code?: string;
  fx_rate?: number | null;
  fx_source?: string | null;
  fx_fetched_at?: string | null;
  status: string; // "pending", "paid", "failed"
  description: string | null;
  due_date: string;
  paid_at: string | null;
  payment_method: string | null;
  external_id?: string | null;
  merchant_id?: string | null;
  proof_attachment?: string | null;
  rejection_reason?: string | null;
  created_at?: string;
  updated_at?: string;
}

export interface FxRate {
  base_currency: string;
  quote_currency: string;
  rate: number;
  source: string;
  fetched_at: string;
}

export interface BulkGenerateInvoicesResult {
  created_count: number;
  skipped_count: number;
  failed_count: number;
}

export interface BillingCollectionRunResult {
  evaluated_count: number;
  reminder_sent_count: number;
  reminder_skipped_count: number;
  suspended_count: number;
  resumed_count: number;
  failed_count: number;
}

export interface BillingCollectionLogView {
  id: string;
  tenant_id: string;
  invoice_id: string;
  subscription_id: string | null;
  action: string;
  result: string;
  reason: string | null;
  actor_type: string;
  actor_id: string | null;
  created_at: string;
  invoice_number: string | null;
  invoice_status: string | null;
  due_date: string | null;
  subscription_status: string | null;
  customer_name: string | null;
}

export interface InvoiceReminderLogView {
  id: string;
  tenant_id: string;
  invoice_id: string;
  reminder_code: string;
  channel: string;
  recipient: string | null;
  status: string;
  detail: string | null;
  created_at: string;
  invoice_number: string | null;
  invoice_status: string | null;
  due_date: string | null;
}

export const payment = {
  listBanks: (): Promise<BankAccount[]> =>
    safeInvoke('list_bank_accounts', { token: getTokenOrThrow() }),

  createBank: (
    bank_name: string,
    account_number: string,
    account_holder: string,
  ): Promise<BankAccount> =>
    safeInvoke('create_bank_account', {
      token: getTokenOrThrow(),
      bankName: bank_name,
      accountNumber: account_number,
      accountHolder: account_holder,
    }),

  deleteBank: (id: string): Promise<void> =>
    safeInvoke('delete_bank_account', { token: getTokenOrThrow(), id }),

  // Invoice & Transaction
  createInvoiceForPlan: (planId: string, billingCycle: 'monthly' | 'yearly'): Promise<Invoice> =>
    safeInvoke('create_invoice_for_plan', { token: getTokenOrThrow(), planId, billingCycle }),

  createInvoiceForCustomerSubscription: (subscriptionId: string): Promise<Invoice> =>
    safeInvoke('create_invoice_for_customer_subscription', {
      token: getTokenOrThrow(),
      subscriptionId,
      subscription_id: subscriptionId,
    }),

  getInvoice: (id: string): Promise<Invoice> =>
    safeInvoke('get_invoice', { token: getTokenOrThrow(), id }),

  listInvoices: (): Promise<Invoice[]> => safeInvoke('list_invoices', { token: getTokenOrThrow() }),

  listCustomerPackageInvoices: (): Promise<Invoice[]> =>
    safeInvoke('list_customer_package_invoices', { token: getTokenOrThrow() }),

  generateDueCustomerPackageInvoices: (): Promise<BulkGenerateInvoicesResult> =>
    safeInvoke('generate_due_customer_package_invoices', { token: getTokenOrThrow() }),

  listBillingCollectionLogs: (filters?: {
    action?: string;
    result?: string;
    from?: string;
    to?: string;
    search?: string;
    limit?: number;
  }): Promise<BillingCollectionLogView[]> =>
    safeInvoke('list_billing_collection_logs', {
      token: getTokenOrThrow(),
      action: filters?.action ?? null,
      result: filters?.result ?? null,
      from: filters?.from ?? null,
      to: filters?.to ?? null,
      search: filters?.search ?? null,
      limit: filters?.limit ?? 200,
    }),

  listInvoiceReminderLogs: (filters?: {
    reminderCode?: string;
    status?: string;
    from?: string;
    to?: string;
    search?: string;
    limit?: number;
  }): Promise<InvoiceReminderLogView[]> =>
    safeInvoke('list_invoice_reminder_logs', {
      token: getTokenOrThrow(),
      reminderCode: filters?.reminderCode ?? null,
      reminder_code: filters?.reminderCode ?? null,
      status: filters?.status ?? null,
      from: filters?.from ?? null,
      to: filters?.to ?? null,
      search: filters?.search ?? null,
      limit: filters?.limit ?? 200,
    }),

  runBillingCollectionNow: (): Promise<BillingCollectionRunResult> =>
    safeInvoke('run_billing_collection_now', { token: getTokenOrThrow() }),

  listAllInvoices: (): Promise<Invoice[]> =>
    safeInvoke('list_all_invoices', { token: getTokenOrThrow() }),

  getFxRate: (baseCurrency: string, quoteCurrency: string): Promise<FxRate> =>
    safeInvoke('get_fx_rate', { token: getTokenOrThrow(), baseCurrency, quoteCurrency }),

  payMidtrans: (id: string): Promise<string> =>
    // Returns Snap Token
    safeInvoke('pay_invoice_midtrans', { token: getTokenOrThrow(), id }),

  checkStatus: (id: string): Promise<string> =>
    safeInvoke('check_payment_status', { token: getTokenOrThrow(), id }),

  submitPaymentProof: (invoiceId: string, filePath: string): Promise<void> =>
    safeInvoke('submit_payment_proof', { token: getTokenOrThrow(), invoiceId, filePath }),

  verifyPayment: (
    invoiceId: string,
    status: 'paid' | 'failed',
    rejectionReason?: string,
  ): Promise<void> =>
    safeInvoke('verify_payment', { token: getTokenOrThrow(), invoiceId, status, rejectionReason }),

  verifyCustomerPackagePayment: (
    invoiceId: string,
    status: 'paid' | 'failed',
    rejectionReason?: string,
  ): Promise<void> =>
    safeInvoke('verify_customer_package_payment', {
      token: getTokenOrThrow(),
      id: invoiceId,
      invoiceId,
      invoice_id: invoiceId,
      status,
      rejectionReason,
      rejection_reason: rejectionReason,
    }),
};

export const storage = {
  listFiles: (
    page: number = 1,
    perPage: number = 20,
    search: string = '',
  ): Promise<PaginatedResponse<FileRecord>> =>
    safeInvoke('list_files_admin', {
      token: getTokenOrThrow(),
      page,
      perPage,
      search: search || null,
    }),

  deleteFile: (fileId: string): Promise<void> =>
    safeInvoke('delete_file_admin', { token: getTokenOrThrow(), fileId }),

  listFilesTenant: (
    page: number = 1,
    perPage: number = 20,
    search: string = '',
  ): Promise<PaginatedResponse<FileRecord>> =>
    safeInvoke('list_files_tenant', {
      token: getTokenOrThrow(),
      page,
      perPage,
      search: search || null,
    }),

  deleteFileTenant: (fileId: string): Promise<void> =>
    safeInvoke('delete_file_tenant', { token: getTokenOrThrow(), fileId }),

  uploadFile: async (
    file: File,
    options?: { paymentInvoiceId?: string | null },
  ): Promise<FileRecord> => {
    const API_BASE = getApiBaseUrl();
    const formData = new FormData();
    formData.append('file', file);

    const query = new URLSearchParams();
    if (options?.paymentInvoiceId) {
      query.set('payment_invoice_id', options.paymentInvoiceId);
    }
    const url = `${API_BASE}/storage/upload${query.toString() ? `?${query.toString()}` : ''}`;

    const response = await fetch(url, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${getTokenOrThrow()}`,
      },
      body: formData,
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

  markAllAsRead: (): Promise<void> => safeInvoke('mark_all_as_read', { token: getTokenOrThrow() }),

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

  sendTest: (): Promise<void> => safeInvoke('send_test', { token: getTokenOrThrow() }),
};

export const emailOutbox = {
  list: (params?: {
    scope?: 'tenant' | 'global' | 'all';
    page?: number;
    perPage?: number;
    status?: string;
    search?: string;
  }): Promise<PaginatedResponse<EmailOutboxItem>> =>
    safeInvoke('list_email_outbox', {
      token: getTokenOrThrow(),
      scope: params?.scope,
      page: params?.page,
      per_page: params?.perPage,
      status: params?.status,
      search: params?.search,
    }),

  get: (id: string): Promise<EmailOutboxItem> =>
    safeInvoke('get_email_outbox', { token: getTokenOrThrow(), id }),

  stats: (scope?: 'tenant' | 'global' | 'all'): Promise<EmailOutboxStats> =>
    safeInvoke('get_email_outbox_stats', { token: getTokenOrThrow(), scope }),

  retry: (id: string): Promise<void> =>
    safeInvoke('retry_email_outbox', { token: getTokenOrThrow(), id }),

  delete: (id: string): Promise<void> =>
    safeInvoke('delete_email_outbox', { token: getTokenOrThrow(), id }),

  retryBulk: (ids: string[]): Promise<{ success: boolean; count: number }> =>
    safeInvoke('bulk_retry_email_outbox', { token: getTokenOrThrow(), ids }),

  deleteBulk: (ids: string[]): Promise<{ success: boolean; count: number }> =>
    safeInvoke('bulk_delete_email_outbox', { token: getTokenOrThrow(), ids }),

  exportCsv: (params?: {
    scope?: 'tenant' | 'global' | 'all';
    status?: string;
    search?: string;
    limit?: number;
  }): Promise<{ csv: string }> =>
    safeInvoke('export_email_outbox_csv', {
      token: getTokenOrThrow(),
      scope: params?.scope,
      status: params?.status,
      search: params?.search,
      limit: params?.limit,
    }),
};

export const backup = {
  list: async (opts?: { scope?: 'all' | 'tenant' }): Promise<BackupRecord[]> => {
    // @ts-ignore
    const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
    const token = getTokenOrThrow();
    const tenantOnly = opts?.scope === 'tenant';
    if (isTauri) {
      return await invoke('list_backups', { args: { token, tenantOnly } });
    }
    return await safeInvoke('list_backups', { token, tenantOnly });
  },

  create: async (backupType: 'global' | 'tenant', targetId?: string): Promise<string> => {
    // @ts-ignore
    const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
    const token = getTokenOrThrow();
    if (isTauri) {
      return await invoke('create_backup', { args: { token, backupType, targetId } });
    }
    return await safeInvoke('create_backup', { token, backupType, targetId });
  },

  delete: async (filename: string): Promise<void> => {
    // @ts-ignore
    const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
    const token = getTokenOrThrow();
    if (isTauri) {
      await invoke('delete_backup', { args: { token, filename } });
      return;
    }
    await safeInvoke('delete_backup', { token, filename });
  },

  download: async (filename: string, onProgress?: (percent: number) => void): Promise<void> => {
    // @ts-ignore
    const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
    const token = getTokenOrThrow();
    const API_BASE = getApiBaseUrl();
    const url = `${API_BASE}/backups/${filename}/download`;

    try {
      const response = await fetch(url, {
        headers: { Authorization: `Bearer ${token}` },
      });

      if (!response.ok) throw new Error(`Download failed: ${response.statusText}`);

      // To track progress, we read the stream manually
      const contentLength = response.headers.get('content-length');
      const total = contentLength ? parseInt(contentLength, 10) : 0;
      let loaded = 0;

      const reader = response.body?.getReader();
      if (!reader) throw new Error('Response body is null');

      const chunks = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
        loaded += value.length;
        if (total > 0 && onProgress) {
          onProgress(Math.round((loaded / total) * 100));
        }
      }

      const blob = new Blob(chunks);

      if (isTauri) {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const { writeFile } = await import('@tauri-apps/plugin-fs');

        const filePath = await save({
          defaultPath: filename,
          filters: [{ name: 'Archive', extensions: ['zip', 'sql'] }],
        });

        if (filePath) {
          await writeFile(filePath, new Uint8Array(await blob.arrayBuffer()));
        }
      } else {
        const downloadUrl = window.URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = downloadUrl;
        link.setAttribute('download', filename);
        document.body.appendChild(link);
        link.click();
        link.remove();
        window.URL.revokeObjectURL(downloadUrl);
      }
    } catch (e: any) {
      console.error('Download error:', e);
      throw e;
    }
  },

  restore: async (file?: File): Promise<void> => {
    // @ts-ignore
    const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;
    const token = getTokenOrThrow();

    if (isTauri && !file) {
      // Tauri path: pick file then call command
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        filters: [{ name: 'Archive', extensions: ['zip'] }],
      });

      if (selected && typeof selected === 'string') {
        return await safeInvoke('restore_backup_from_file', { token, path: selected });
      }
      throw new Error('No file selected');
    } else if (file) {
      // Web path: upload via HTTP multipart
      const API_BASE = getApiBaseUrl();
      const formData = new FormData();
      formData.append('file', file);

      const response = await fetch(`${API_BASE}/backups/restore`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${token}` },
        body: formData,
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.error || 'Restore failed');
      }
    } else {
      throw new Error('File required for web restore');
    }
  },

  restoreLocal: async (filename: string): Promise<void> => {
    const token = getTokenOrThrow();
    return await safeInvoke('restore_local_backup_command', { token, filename });
  },
};

// Combined API object
export const api = {
  auth,
  users,
  roles,
  team,
  customers,
  workOrders,
  pppoe,
  ispPackages,
  networkMapping,
  superadmin,
  audit,
  mikrotik,
  support,
  announcements,
  settings,
  install,
  plans,
  storage,
  payment,
  tenant,
  notifications,
  emailOutbox,
  backup,
};

export default api;
