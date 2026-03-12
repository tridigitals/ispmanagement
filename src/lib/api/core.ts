import { invoke } from '@tauri-apps/api/core';
import { getApiBaseUrl } from '$lib/utils/apiUrl';

const AUTH_STORAGE_KEYS = ['auth_token', 'auth_user', 'auth_tenant', 'active_tenant_slug'] as const;

let authRedirectInProgress = false;

const commandMap: Record<string, { method: string; path: string }> = {
  is_installed: { method: 'GET', path: '/install/check' },
  install_app: { method: 'POST', path: '/install' },
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
  list_users: { method: 'GET', path: '/users' },
  get_user: { method: 'GET', path: '/users/:id' },
  create_user: { method: 'POST', path: '/users' },
  update_user: { method: 'PUT', path: '/users/:id' },
  delete_user: { method: 'DELETE', path: '/users/:id' },
  list_my_addresses: { method: 'GET', path: '/users/me/addresses' },
  create_my_address: { method: 'POST', path: '/users/me/addresses' },
  update_my_address: { method: 'PUT', path: '/users/me/addresses/:addressId' },
  delete_my_address: { method: 'DELETE', path: '/users/me/addresses/:addressId' },
  list_tenants: { method: 'GET', path: '/superadmin/tenants' },
  create_tenant: { method: 'POST', path: '/superadmin/tenants' },
  delete_tenant: { method: 'DELETE', path: '/superadmin/tenants/:id' },
  list_audit_logs: { method: 'GET', path: '/superadmin/audit-logs' },
  get_system_health: { method: 'GET', path: '/superadmin/system' },
  get_system_diagnostics: { method: 'GET', path: '/superadmin/diagnostics' },
  list_support_tickets: { method: 'GET', path: '/support/tickets' },
  get_support_ticket_stats: { method: 'GET', path: '/support/tickets/stats' },
  create_support_ticket: { method: 'POST', path: '/support/tickets' },
  get_support_ticket: { method: 'GET', path: '/support/tickets/:id' },
  reply_support_ticket: { method: 'POST', path: '/support/tickets/:id/messages' },
  update_support_ticket: { method: 'PUT', path: '/support/tickets/:id' },
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
  update_my_customer_location: {
    method: 'PUT',
    path: '/customers/portal/my-locations/:locationId',
  },
  delete_my_customer_location: {
    method: 'DELETE',
    path: '/customers/portal/my-locations/:locationId',
  },
  list_my_customer_packages: { method: 'GET', path: '/customers/portal/my-packages' },
  get_my_customer_subscription_stats: {
    method: 'GET',
    path: '/customers/portal/my-subscriptions/stats',
  },
  list_my_customer_subscriptions: { method: 'GET', path: '/customers/portal/my-subscriptions' },
  create_my_customer_subscription_order_request: {
    method: 'POST',
    path: '/customers/portal/order-request',
  },
  reopen_my_customer_subscription_order_request: {
    method: 'POST',
    path: '/customers/portal/my-subscriptions/:subscriptionId/reopen-request',
  },
  get_my_customer_subscription_installation_tracker: {
    method: 'GET',
    path: '/customers/portal/my-subscriptions/:subscriptionId/installation-tracker',
  },
  request_my_customer_subscription_reschedule: {
    method: 'POST',
    path: '/customers/portal/my-subscriptions/:subscriptionId/reschedule-request',
  },
  create_my_customer_subscription_invoice: {
    method: 'POST',
    path: '/customers/portal/checkout',
  },
  list_installation_work_orders: { method: 'GET', path: '/admin/work-orders' },
  list_installation_assignees: { method: 'GET', path: '/admin/work-orders/assignees' },
  assign_installation_work_order: { method: 'POST', path: '/admin/work-orders/:id/assign' },
  claim_installation_work_order: { method: 'POST', path: '/admin/work-orders/:id/claim' },
  release_installation_work_order: { method: 'POST', path: '/admin/work-orders/:id/release' },
  start_installation_work_order: { method: 'POST', path: '/admin/work-orders/:id/start' },
  complete_installation_work_order: { method: 'POST', path: '/admin/work-orders/:id/complete' },
  cancel_installation_work_order: { method: 'POST', path: '/admin/work-orders/:id/cancel' },
  reopen_installation_work_order: { method: 'POST', path: '/admin/work-orders/:id/reopen' },
  get_pending_work_order_reschedule_request: {
    method: 'GET',
    path: '/admin/work-orders/:id/reschedule-request',
  },
  approve_work_order_reschedule_request: {
    method: 'POST',
    path: '/admin/work-orders/:id/reschedule-request/approve',
  },
  reject_work_order_reschedule_request: {
    method: 'POST',
    path: '/admin/work-orders/:id/reschedule-request/reject',
  },
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
  list_team_members: { method: 'GET', path: '/team' },
  add_team_member: { method: 'POST', path: '/team' },
  update_team_member_role: { method: 'PUT', path: '/team/:memberId' },
  remove_team_member: { method: 'DELETE', path: '/team/:memberId' },
  get_roles: { method: 'GET', path: '/roles' },
  get_role: { method: 'GET', path: '/roles/:id' },
  create_new_role: { method: 'POST', path: '/roles' },
  update_existing_role: { method: 'PUT', path: '/roles/:id' },
  delete_existing_role: { method: 'DELETE', path: '/roles/:id' },
  get_permissions: { method: 'GET', path: '/permissions' },
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
  list_files_admin: { method: 'GET', path: '/storage/files' },
  list_files_tenant: { method: 'GET', path: '/storage/files' },
  delete_file_admin: { method: 'DELETE', path: '/storage/files/:file_id' },
  delete_file_tenant: { method: 'DELETE', path: '/storage/files/:file_id' },
  upload_init: { method: 'POST', path: '/storage/upload/init' },
  upload_chunk: { method: 'POST', path: '/storage/upload/chunk' },
  upload_complete: { method: 'POST', path: '/storage/upload/complete' },
  list_bank_accounts: { method: 'GET', path: '/payment/banks' },
  create_bank_account: { method: 'POST', path: '/payment/banks' },
  delete_bank_account: { method: 'DELETE', path: '/payment/banks/:id' },
  create_invoice_for_plan: { method: 'POST', path: '/payment/invoices/plan' },
  create_invoice_for_customer_subscription: {
    method: 'POST',
    path: '/payment/invoices/customer-package/create',
  },
  create_invoice_for_installation_work_order: {
    method: 'POST',
    path: '/payment/invoices/installation/create',
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
  verify_payment: { method: 'POST', path: '/payment/invoices/:invoiceId/verify' },
  list_all_invoices: { method: 'GET', path: '/payment/invoices/all' },
  get_fx_rate: { method: 'GET', path: '/payment/fx-rate' },
  pay_invoice_midtrans: { method: 'POST', path: '/payment/invoices/:id/midtrans' },
  check_payment_status: { method: 'GET', path: '/payment/invoices/:id/status' },
  submit_payment_proof: { method: 'POST', path: '/payment/invoices/:invoiceId/proof' },
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
  list_email_outbox: { method: 'GET', path: '/email-outbox' },
  get_email_outbox: { method: 'GET', path: '/email-outbox/:id' },
  get_email_outbox_stats: { method: 'GET', path: '/email-outbox/stats' },
  retry_email_outbox: { method: 'POST', path: '/email-outbox/:id/retry' },
  delete_email_outbox: { method: 'DELETE', path: '/email-outbox/:id' },
  bulk_retry_email_outbox: { method: 'POST', path: '/email-outbox/bulk/retry' },
  bulk_delete_email_outbox: { method: 'POST', path: '/email-outbox/bulk/delete' },
  export_email_outbox_csv: { method: 'GET', path: '/email-outbox/export' },
  list_active_announcements: { method: 'GET', path: '/announcements/active' },
  list_recent_announcements: { method: 'GET', path: '/announcements/recent' },
  get_announcement: { method: 'GET', path: '/announcements/:id' },
  dismiss_announcement: { method: 'POST', path: '/announcements/:id/dismiss' },
  list_announcements_admin: { method: 'GET', path: '/announcements/admin' },
  create_announcement_admin: { method: 'POST', path: '/announcements/admin' },
  update_announcement_admin: { method: 'PUT', path: '/announcements/admin/:id' },
  delete_announcement_admin: { method: 'DELETE', path: '/announcements/admin/:id' },
  get_current_tenant: { method: 'GET', path: '/tenant/me' },
  update_current_tenant: { method: 'PUT', path: '/tenant/me' },
  list_tenant_audit_logs: { method: 'GET', path: '/admin/audit-logs' },
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
  get_mikrotik_router_snapshot: {
    method: 'GET',
    path: '/admin/mikrotik/routers/:id/snapshot',
  },
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
  list_mikrotik_ip_pools: { method: 'GET', path: '/admin/mikrotik/routers/:routerId/ip-pools' },
  sync_mikrotik_ip_pools: {
    method: 'POST',
    path: '/admin/mikrotik/routers/:routerId/ip-pools/sync',
  },
  sync_mikrotik_logs: { method: 'POST', path: '/admin/mikrotik/routers/:routerId/logs/sync' },
  list_pppoe_accounts: { method: 'GET', path: '/admin/pppoe/accounts' },
  get_pppoe_account: { method: 'GET', path: '/admin/pppoe/accounts/:id' },
  create_pppoe_account: { method: 'POST', path: '/admin/pppoe/accounts' },
  update_pppoe_account: { method: 'PUT', path: '/admin/pppoe/accounts/:id' },
  delete_pppoe_account: { method: 'DELETE', path: '/admin/pppoe/accounts/:id' },
  apply_pppoe_account: { method: 'POST', path: '/admin/pppoe/accounts/:id/apply' },
  reconcile_pppoe_router: { method: 'POST', path: '/admin/pppoe/routers/:routerId/reconcile' },
  preview_pppoe_import_from_router: {
    method: 'GET',
    path: '/admin/pppoe/routers/:routerId/import/preview',
  },
  import_pppoe_from_router: { method: 'POST', path: '/admin/pppoe/routers/:routerId/import' },
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
  list_network_nodes: { method: 'GET', path: '/admin/network-mapping/nodes' },
  create_network_node: { method: 'POST', path: '/admin/network-mapping/nodes' },
  update_network_node: { method: 'PATCH', path: '/admin/network-mapping/nodes/:id' },
  delete_network_node: { method: 'DELETE', path: '/admin/network-mapping/nodes/:id' },
  list_network_links: { method: 'GET', path: '/admin/network-mapping/links' },
  create_network_link: { method: 'POST', path: '/admin/network-mapping/links' },
  connect_network_node_to_link: {
    method: 'POST',
    path: '/admin/network-mapping/links/connect-node-to-link',
  },
  update_network_link: { method: 'PATCH', path: '/admin/network-mapping/links/:id' },
  delete_network_link: { method: 'DELETE', path: '/admin/network-mapping/links/:id' },
  list_service_zones: { method: 'GET', path: '/admin/network-mapping/zones' },
  create_service_zone: { method: 'POST', path: '/admin/network-mapping/zones' },
  update_service_zone: { method: 'PATCH', path: '/admin/network-mapping/zones/:id' },
  delete_service_zone: { method: 'DELETE', path: '/admin/network-mapping/zones/:id' },
  resolve_service_zone: { method: 'POST', path: '/admin/network-mapping/zones/resolve' },
  compute_network_path: { method: 'POST', path: '/admin/network-mapping/paths/compute' },
  sync_network_mapping_assets: { method: 'POST', path: '/admin/network-mapping/assets/sync' },
  rank_candidate_network_nodes: {
    method: 'POST',
    path: '/admin/network-mapping/nodes/rank-candidates',
  },
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
  list_network_impacted_customers: {
    method: 'GET',
    path: '/admin/network-mapping/impact/customers',
  },
  list_backups: { method: 'GET', path: '/backups' },
  create_backup: { method: 'POST', path: '/backups' },
  delete_backup: { method: 'DELETE', path: '/backups/:filename' },
};

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
  if (!hasStoredAuthToken()) return;
  if (authRedirectInProgress) return;
  authRedirectInProgress = true;

  clearStoredAuthData();

  try {
    window.dispatchEvent(new CustomEvent('app:auth-expired', { detail: { reason } }));
  } catch {
    // non-blocking
  }

  setTimeout(() => {
    if (window.location.pathname.startsWith('/login')) return;
    window.location.assign('/login?reason=expired');
  }, 0);
}

export function isTauriRuntime(): boolean {
  if (typeof window === 'undefined') return false;
  const w = window as any;
  return (
    !!w.__TAURI_INTERNALS__ ||
    !!w.__TAURI__ ||
    (typeof navigator !== 'undefined' &&
      typeof navigator.userAgent === 'string' &&
      navigator.userAgent.toLowerCase().includes('tauri'))
  );
}

export async function safeInvoke<T>(command: string, args?: any): Promise<T> {
  try {
    const forceRemote = import.meta.env.VITE_USE_REMOTE_API === 'true';
    const looksLikeTauri = isTauriRuntime();

    if (looksLikeTauri && command === 'register') {
      throw new Error(
        'Pendaftaran akun hanya tersedia melalui web browser pada domain/workspace tenant yang benar.',
      );
    }

    if (!forceRemote && typeof window !== 'undefined' && looksLikeTauri) {
      try {
        return await invoke(command, args);
      } catch (e: any) {
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

    const API_BASE = getApiBaseUrl();
    const route = commandMap[command];

    if (route) {
      const toSnakeCase = (str: string) =>
        str.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
      const toCamelCase = (str: string) =>
        str.replace(/_([a-z])/g, (_m, c: string) => c.toUpperCase());

      let path = route.path;
      const queryParams: Record<string, string> = {};
      const consumedPathKeys = new Set<string>();

      if (args) {
        for (const [key, value] of Object.entries(args)) {
          if (key.startsWith('__')) continue;
          if (value === null || value === undefined) continue;

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
            queryParams[snakeKey] = String(value);
          }
        }
      }

      const queryString =
        Object.keys(queryParams).length > 0
          ? '?' + new URLSearchParams(queryParams).toString()
          : '';

      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
      };

      const token =
        args?.token || localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
      if (token) {
        headers.Authorization = `Bearer ${token}`;
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
          if (abortedByExternalSignal) throw new Error('Request canceled');
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

      if (response.status === 204) return undefined as T;

      const raw = await response.text();
      if (!raw || !raw.trim()) return undefined as T;
      if (isJson) return JSON.parse(raw) as T;

      return raw as T;
    }

    console.warn(`[Mock] Calling ${command} with`, args);
    if (command === 'get_current_user') return null as any;
    if (command === 'get_all_settings') return [] as any;
    if (command === 'list_users') return { data: [], total: 0, page: 1, per_page: 10 } as any;
    if (command === 'validate_token') return true as any;
    if (command === 'is_installed') return false as any;
    if (command === 'get_auth_settings') {
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
    }

    throw new Error(`Command '${command}' not implemented in HTTP API yet.`);
  } catch (error: any) {
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

export function getTokenOrThrow(): string {
  if (typeof window === 'undefined') throw new Error('Client side only');
  const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
  if (!token) throw new Error('Authentication required');
  return token;
}
