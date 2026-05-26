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
  custom_domain_status?: string | null;
  custom_domain_verified_at?: string | null;
  custom_domain_failure_reason?: string | null;
  logo_url?: string;
  is_active: boolean;
  enforce_2fa: boolean;
  created_at: string;
  updated_at: string;
}

export interface SuperadminManagedRadiusServer {
  id: string;
  name: string;
  endpoint_host: string;
  auth_port: number;
  acct_port: number;
  is_active: boolean;
  is_default: boolean;
  notes: string | null;
  tenant_count: number;
  router_count: number;
  updated_at: string;
}

export interface SuperadminManagedRadiusRuntimeStatus {
  enabled: boolean;
  running: boolean;
  bind_addr: string;
  auth_port: number;
  acct_port: number;
  advertised_host: string;
  require_message_authenticator: boolean;
}

export interface SuperadminManagedRadiusAssignment {
  id: string;
  tenant_id: string;
  tenant_name: string;
  radius_endpoint_id: string;
  endpoint_name: string;
  radius_host: string;
  auth_port: number;
  acct_port: number;
  is_active: boolean;
  router_count: number;
  updated_at: string;
}

export interface SuperadminManagedRadiusMapping {
  id: string;
  tenant_id: string;
  tenant_name: string;
  radius_endpoint_id: string;
  endpoint_name: string;
  radius_host: string;
  auth_port: number;
  acct_port: number;
  router_id: string;
  router_name: string | null;
  nas_name: string;
  nas_ip_or_cidr: string;
  shortname: string | null;
  shared_secret_masked: string;
  is_active: boolean;
  updated_at: string;
}

export interface SuperadminManagedRadiusUser {
  id: string;
  tenant_id: string;
  tenant_name: string;
  router_id: string;
  router_name: string | null;
  username: string;
  radius_identity: string | null;
  account_source: 'managed_radius' | 'router' | string;
  is_provisioned: boolean;
  provisioned_at: string | null;
  provisioning_error: string | null;
  router_profile_name: string | null;
  updated_at: string;
}

export interface SuperadminManagedRadiusSession {
  id: string;
  tenant_id: string;
  tenant_name: string;
  router_id: string;
  router_name: string | null;
  username: string;
  radius_identity: string | null;
  acct_session_id: string;
  status_type: string;
  framed_ip_address: string | null;
  calling_station_id: string | null;
  session_time_seconds: number | null;
  input_octets: number | null;
  output_octets: number | null;
  started_at: string | null;
  last_update_at: string | null;
  ended_at: string | null;
  updated_at: string;
}

export interface ManagedRadiusServerPayload {
  name: string;
  endpoint_host: string;
  endpoint_port?: number | null;
  runtime_label?: string | null;
  runtime_user?: string | null;
  runtime_secret?: string | null;
  is_active: boolean;
  notes?: string | null;
}

export interface ManagedRadiusAssignmentPayload {
  tenant_id: string;
  radius_endpoint_id: string;
  is_active: boolean;
}

export interface ManagedRadiusMappingPayload {
  tenant_id: string;
  radius_endpoint_id: string;
  router_id: string;
  nas_name: string;
  nas_ip_or_cidr: string;
  shortname?: string | null;
  shared_secret?: string | null;
  is_active: boolean;
}

export interface ManagedRadiusSecretValue {
  shared_secret: string;
  shared_secret_masked: string;
}

export interface AuthResponse {
  user: User;
  tenant?: Tenant;
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
  permissions?: string[];
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

export interface CustomerListItem extends Customer {
  subscription_count: number;
  active_subscriptions: number;
  pending_installations: number;
  service_status: 'active' | 'inactive' | 'none' | string;
}

export interface CustomerSummary {
  total: number;
  active: number;
  inactive: number;
  pending_installation: number;
}

export interface CustomerServiceLifecycleIssue {
  issue_type: string;
  customer_id: string;
  customer_name: string;
  subscription_id: string;
  subscription_status: string;
  package_name: string | null;
  location_label: string | null;
  starts_at: string | null;
  ends_at: string | null;
  recommended_action: string;
}

export interface CustomerServiceLifecycleReport {
  generated_at: string;
  total_issues: number;
  missing_bootstrap_invoice: number;
  invalid_active_lifecycle: number;
  page: number;
  per_page: number;
  data: CustomerServiceLifecycleIssue[];
}

export interface CustomerServiceLifecycleRepairResult {
  issue_type: string;
  matched_count: number;
  repaired_count: number;
  skipped_count: number;
  failed_count: number;
  errors: string[];
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

export type NetworkAssetType =
  | 'olt'
  | 'odc'
  | 'odp'
  | 'splitter'
  | 'ont'
  | 'onu'
  | 'fat'
  | 'nap'
  | 'switch'
  | 'router'
  | 'media_converter'
  | 'odf'
  | 'ups';

export type NetworkAssetGroup = 'access_fiber' | 'infrastructure' | string;

export type NetworkAssetStatus = 'available' | 'reserved' | 'installed' | 'faulty' | 'retired';

export interface NetworkAsset {
  id: string;
  tenant_id: string;
  asset_group: NetworkAssetGroup;
  asset_type: NetworkAssetType | string;
  name: string;
  code: string | null;
  vendor: string | null;
  model: string | null;
  serial_number: string | null;
  status: NetworkAssetStatus | string;
  customer_id: string | null;
  location_id: string | null;
  work_order_id: string | null;
  parent_asset_id: string | null;
  latitude: number | null;
  longitude: number | null;
  notes: string | null;
  metadata: Record<string, unknown> | null;
  created_at: string;
  updated_at: string;
}

export interface NetworkAssetListItem extends NetworkAsset {
  customer_name: string | null;
  location_label: string | null;
  work_order_status: string | null;
  parent_asset_name: string | null;
}

export interface CreateNetworkAssetRequest {
  asset_type: NetworkAssetType | string;
  name: string;
  code?: string | null;
  vendor?: string | null;
  model?: string | null;
  serial_number?: string | null;
  status?: NetworkAssetStatus | string;
  customer_id?: string | null;
  location_id?: string | null;
  work_order_id?: string | null;
  parent_asset_id?: string | null;
  latitude?: number | null;
  longitude?: number | null;
  notes?: string | null;
  metadata?: Record<string, unknown> | null;
}

export interface UpdateNetworkAssetRequest {
  asset_type?: NetworkAssetType | string;
  name?: string;
  code?: string | null;
  vendor?: string | null;
  model?: string | null;
  serial_number?: string | null;
  status?: NetworkAssetStatus | string;
  customer_id?: string | null;
  location_id?: string | null;
  work_order_id?: string | null;
  parent_asset_id?: string | null;
  latitude?: number | null;
  longitude?: number | null;
  notes?: string | null;
  metadata?: Record<string, unknown> | null;
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
  invite_url?: string | null;
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
  status:
    | 'active'
    | 'grace_active'
    | 'pending_installation'
    | 'installation_done_awaiting_payment'
    | 'suspended'
    | 'cancelled'
    | string;
  starts_at: string | null;
  ends_at: string | null;
  grace_started_at: string | null;
  grace_until: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface CustomerSubscriptionView extends CustomerSubscription {
  package_name: string | null;
  location_label: string | null;
  router_name: string | null;
  pppoe_address_pool: string | null;
  pppoe_isolation_pool: string | null;
  pppoe_disabled: boolean | null;
  latest_work_order_id: string | null;
  latest_work_order_status: string | null;
  can_request_reopen: boolean;
  latest_reschedule_status: string | null;
  latest_reschedule_requested_at: string | null;
}

export interface CustomerPortalSubscriptionStats {
  total: number;
  active: number;
  pending_installation: number;
  needs_attention: number;
}

export interface CustomerLifecycleStageMetric {
  stage: string;
  count: number;
}

export interface CustomerLifecycleAgingBucket {
  bucket: string;
  count: number;
}

export interface CustomerLifecycleObservability {
  generated_at: string;
  lifecycle_funnel: CustomerLifecycleStageMetric[];
  work_order_funnel: CustomerLifecycleStageMetric[];
  aging_buckets: CustomerLifecycleAgingBucket[];
}

export interface InstallationWorkOrder {
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
}

export interface InstallationWorkOrderView {
  id: string;
  tenant_id: string;
  subscription_id: string;
  invoice_id: string | null;
  customer_id: string;
  location_id: string;
  package_id: string | null;
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
  package_provisioning_type: 'pppoe' | 'dhcp_static' | string | null;
  router_name: string | null;
  assigned_to_name: string | null;
  assigned_to_email: string | null;
  assignment_id: string | null;
  assignment_status: string | null;
  subscription_status: string | null;
  subscription_starts_at: string | null;
  subscription_grace_until: string | null;
  has_customer_package_invoice: boolean;
  selected_zone_id: string | null;
  selected_zone_name: string | null;
  selected_node_id: string | null;
  selected_node_name: string | null;
  selected_node_score: number | null;
  path_node_ids: unknown[] | null;
  path_link_ids: unknown[] | null;
}

export interface WorkOrderRescheduleRequestView {
  id: string;
  work_order_id: string;
  requested_schedule_at: string;
  reason: string | null;
  status: 'pending' | 'approved' | 'rejected' | 'cancelled' | string;
  requested_by_name: string | null;
  requested_by_email: string | null;
  reviewed_by_name: string | null;
  reviewed_at: string | null;
  review_notes: string | null;
  created_at: string;
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
  status: string;
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

export interface CustomerPortalCheckoutResponse {
  subscription: CustomerSubscription;
  invoice: Invoice;
}

export interface CustomerPortalOrderRequestResponse {
  subscription: CustomerSubscription;
  work_order: InstallationWorkOrderView;
}

export interface CustomerPortalInstallationTrackerResponse {
  subscription: CustomerSubscriptionView;
  work_order: InstallationWorkOrderView | null;
  reschedule_request: WorkOrderRescheduleRequestView | null;
}

export type BackofficeOrderCustomerMode = 'new' | 'existing';
export type BackofficeOrderLocationMode = 'new' | 'existing';

export interface BackofficeOrderCustomerInput {
  name: string;
  email?: string | null;
  phone?: string | null;
  notes?: string | null;
  is_active?: boolean;
}

export interface BackofficeOrderLocationInput {
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
}

export interface CreateBackofficeInstallationOrderRequest {
  customer_mode: BackofficeOrderCustomerMode;
  customer_id?: string | null;
  customer?: BackofficeOrderCustomerInput | null;
  location_mode: BackofficeOrderLocationMode;
  location_id?: string | null;
  location?: BackofficeOrderLocationInput | null;
  package_id: string;
  billing_cycle?: 'monthly' | 'yearly' | string | null;
  notes?: string | null;
  requested_installation_date?: string | null;
}

export interface BackofficeInstallationOrderResponse {
  customer: Customer;
  location: CustomerLocation;
  subscription: CustomerSubscription;
  work_order: InstallationWorkOrder;
}

export interface CustomerSubscriptionOption {
  id: string;
  customer_id: string;
  customer_name: string;
  package_name?: string | null;
  billing_cycle: string;
  status: string;
}

export interface PppoeAccountPublic {
  account_source: 'router' | 'managed_radius';
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
  is_provisioned: boolean;
  radius_identity: string | null;
  provisioned_at: string | null;
  provisioning_error: string | null;
  created_at: string;
  updated_at: string;
}

export interface DhcpStaticServicePublic {
  id: string;
  tenant_id: string;
  subscription_id: string;
  router_id: string;
  customer_id: string;
  location_id: string;
  package_id: string;
  dhcp_server_name: string;
  mac_address: string;
  ip_address: string;
  comment: string | null;
  disabled: boolean;
  lease_present: boolean;
  lease_router_ref: string | null;
  lease_last_sync_at: string | null;
  lease_last_error: string | null;
  queue_mode: 'none' | 'simple_queue' | string;
  queue_name: string | null;
  queue_target: string | null;
  queue_rate_limit: string | null;
  queue_present: boolean;
  queue_last_sync_at: string | null;
  queue_last_error: string | null;
  created_at: string;
  updated_at: string;
}

export interface ManagedRadiusRouterSetup {
  configured: boolean;
  router_id: string;
  plan_allows_managed_radius: boolean;
  plan_upgrade_required: boolean;
  upgrade_path: string | null;
  tenant_has_active_assignment: boolean;
  default_server_available: boolean;
  can_assign_default: boolean;
  can_create_mapping: boolean;
  assignment_endpoint_name: string | null;
  endpoint_name: string | null;
  radius_host: string | null;
  auth_port: number;
  acct_port: number;
  nas_ip_or_cidr: string | null;
  shared_secret: string | null;
  shared_secret_masked: string | null;
  cli_script: string | null;
  warnings: string[];
}

export interface IspPackage {
  id: string;
  tenant_id: string;
  service_type: string;
  provisioning_type: 'pppoe' | 'dhcp_static' | string;
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
  router_name: string | null;
  router_profile_name: string;
  address_pool: string | null;
  isolation_pool: string | null;
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

export interface BankAccount {
  id: string;
  bank_name: string;
  account_number: string;
  account_holder: string;
  is_active: boolean;
}

export interface DuitkuPaymentMethod {
  code: string;
  name: string;
  fee?: string | null;
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
  channel: 'in_app' | 'email' | 'push' | 'whatsapp';
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
  last_attempted_at: string | null;
  next_retry_at: string | null;
  retryable: boolean;
  delivery_status_summary: string;
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
  customer_name: string | null;
}

export type WhatsAppGatewayProvider = 'disabled' | 'fonnte' | 'triwax';

export interface WhatsAppGatewayConfig {
  enabled: boolean;
  provider: WhatsAppGatewayProvider;
  fonnte_token?: string | null;
  fonnte_base_url?: string | null;
  fonnte_sender?: string | null;
  triwax_api_key?: string | null;
}

export interface WhatsAppGatewayFormState {
  enabled: boolean;
  provider: WhatsAppGatewayProvider;
  fonnteToken: string;
  fonnteBaseUrl: string;
  fonnteSender: string;
  triwaxApiKey: string;
}

export type WhatsAppGatewaySettingsMap = Record<string, string | null | undefined>;

export type WhatsAppEventScope = 'platform' | 'tenant';

export interface WhatsAppEventDefinition {
  code: string;
  label: string;
  scope: WhatsAppEventScope;
  description?: string | null;
}

export interface WhatsAppEventPreference {
  whatsapp: boolean;
  email: boolean;
  in_app: boolean;
}

export interface WhatsAppTestSendRequest {
  phone: string;
  message: string;
  eventCode?: string;
}

export interface WhatsAppTestSendResponse {
  ok: boolean;
  provider: WhatsAppGatewayProvider | string;
  message_id?: string | null;
  status?: number | null;
  error?: string | null;
}

export interface WhatsAppGatewayReadiness {
  ready: boolean;
  provider: WhatsAppGatewayProvider | string;
  reason?: string | null;
}

export interface WhatsAppCustomerSendRequest {
  customerId: string;
  message: string;
  template?: string;
  templateId?: string | null;
}

export interface CustomerEmailSendRequest {
  customerId: string;
  subject: string;
  body: string;
  templateId?: string | null;
}

export interface CustomerEmailSendResponse {
  ok: boolean;
  queued: boolean;
}

export type MessageTemplateChannel = 'whatsapp' | 'email' | 'both';
export type MessageTemplateStatus = 'draft' | 'active' | 'archived';
export type MessageTemplateTriggerMode = 'manual' | 'automatic' | 'both';

export interface MessageTemplate {
  id: string;
  tenant_id: string;
  key: string;
  name: string;
  description?: string | null;
  use_case: string;
  target: string;
  trigger_mode: MessageTemplateTriggerMode | string;
  event_key?: string | null;
  channel: MessageTemplateChannel | string;
  locale: string;
  status: MessageTemplateStatus | string;
  whatsapp_body?: string | null;
  email_subject?: string | null;
  email_body?: string | null;
  variables: string[];
  version: number;
  created_at: string;
  updated_at: string;
}

export interface MessageTemplateListFilters {
  q?: string;
  useCase?: string;
  channel?: MessageTemplateChannel | 'all';
  status?: MessageTemplateStatus | 'all';
  target?: string;
  triggerMode?: MessageTemplateTriggerMode | 'all';
}

export interface MessageTemplatePayload {
  key: string;
  name: string;
  description?: string | null;
  useCase: string;
  target: string;
  triggerMode: MessageTemplateTriggerMode | string;
  eventKey?: string | null;
  channel: MessageTemplateChannel | string;
  locale?: string | null;
  status: MessageTemplateStatus | string;
  whatsappBody?: string | null;
  emailSubject?: string | null;
  emailBody?: string | null;
}

export interface MessageTemplatePreviewRequest {
  whatsappBody?: string | null;
  emailSubject?: string | null;
  emailBody?: string | null;
  context: Record<string, any>;
}

export interface MessageTemplatePreviewResponse {
  whatsappBody?: string | null;
  emailSubject?: string | null;
  emailBody?: string | null;
  variables: string[];
}
