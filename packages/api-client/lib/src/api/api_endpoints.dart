/// API endpoints constants for the ISP Management backend.
///
/// Maps to actual Axum routes in src-tauri/src/http/*.rs
/// Mounted at: /api/customers (portal), /api/auth, /api/payment, /api/notifications
class ApiEndpoints {
  ApiEndpoints._();

  // ── Auth (src-tauri/src/http/auth.rs) ──
  static const String authLogin = '/api/auth/login';
  static const String authRegister = '/api/auth/register';
  /// NOTE: Backend has no logout endpoint. Logout is handled client-side only.
  static const String authLogout = '/api/auth/logout'; // unused — kept for reference
  static const String authMe = '/api/auth/me';
  static const String authValidate = '/api/auth/validate';
  /// NOTE: Backend has no refresh endpoint. Tokens cannot be refreshed.
  static const String authRefresh = '/api/auth/refresh'; // unused — kept for reference
  static const String authForgotPassword = '/api/auth/forgot-password';
  static const String authResetPassword = '/api/auth/reset-password';
  /// NOTE: Backend has no change-password endpoint. Not implemented.
  static const String authChangePassword = '/api/auth/change-password';
  static const String auth2faVerify = '/api/auth/2fa/verify';
  static const String auth2faEnable = '/api/auth/2fa/enable';

  // Temp token 2FA setup (forced enrollment — uses temp_token, not JWT)
  static const String auth2faTempEnable = '/api/auth/2fa/temp/enable';
  static const String auth2faTempVerifySetup =
      '/api/auth/2fa/temp/verify-setup';
  static const String auth2faTempEmailEnableRequest =
      '/api/auth/2fa/temp/email/enable-request';
  static const String auth2faTempEmailEnableVerify =
      '/api/auth/2fa/temp/email/enable-verify';

  // ── Portal / Me ──
  // Backend: /api/auth/me returns user info (no separate /api/portal/me)
  static const String me = '/api/auth/me';

  // ── Customer Portal Subscriptions ──
  // Backend: /api/customers/portal/my-subscriptions (src-tauri/src/http/customers.rs)
  static const String mySubscriptions = '/api/customers/portal/my-subscriptions';
  static String mySubscriptionById(String id) =>
      '/api/customers/portal/my-subscriptions/$id';
  static const String mySubscriptionsStats =
      '/api/customers/portal/my-subscriptions/stats';
  static String mySubscriptionInstallationTracker(String id) =>
      '/api/customers/portal/my-subscriptions/$id/installation-tracker';

  // ── Customer Portal Packages ──
  static const String myPackages = '/api/customers/portal/my-packages';

  // ── Customer Portal Locations ──
  static const String myLocations = '/api/customers/portal/my-locations';
  static String myLocationById(String id) =>
      '/api/customers/portal/my-locations/$id';

  // ── Invoices / Payments ──
  // Backend: /api/payment/invoices (src-tauri/src/http/payment.rs)
  static const String myInvoices = '/api/payment/invoices';
  static String myInvoiceById(String id) => '/api/payment/invoices/$id';
  static String invoicePaymentChannels(String invoiceId) =>
      '/api/payment/invoices/$invoiceId/payment-channels';
  static String payInvoiceMidtrans(String invoiceId) =>
      '/api/payment/invoices/$invoiceId/midtrans';
  static String payInvoiceDuitku(String invoiceId) =>
      '/api/payment/invoices/$invoiceId/duitku';
  static String checkPaymentStatus(String invoiceId) =>
      '/api/payment/invoices/$invoiceId/status';
  static String submitPaymentProof(String invoiceId) =>
      '/api/payment/invoices/$invoiceId/proof';
  static const String duitkuPaymentMethods =
      '/api/payment/duitku/payment-methods';

  // ── Support Tickets ──
  // Backend: /api/support/tickets (src-tauri/src/bootstrap/http.rs)
  static const String myTickets = '/api/support/tickets';
  static String myTicketById(String id) => '/api/support/tickets/$id';
  static String ticketMessages(String id) =>
      '/api/support/tickets/$id/messages';
  static const String createTicket = '/api/support/tickets';
  static String ticketSatisfaction(String id) =>
      '/api/support/tickets/$id/satisfaction';

  // ── Storage / Files ──
  // Backend: /api/storage/files/{id}/content (src-tauri/src/http/storage.rs)
  /// URL to serve file content inline (for images in WebView, etc.)
  static String fileContent(String fileId) =>
      '/api/storage/files/$fileId/content';
  /// URL to download file as attachment.
  static String fileDownload(String fileId) =>
      '/api/storage/files/$fileId/download';
  /// URL for ticket attachment content (customer-accessible, no storage_files:read needed).
  static String ticketAttachmentContent(String fileId) =>
      '/api/storage/files/$fileId/ticket-content';

  // ── Notifications ──
  // Backend: /api/notifications (src-tauri/src/http/notifications.rs)
  static const String notifications = '/api/notifications';
  static const String notificationsUnreadCount =
      '/api/notifications/unread-count';
  static const String notificationsReadAll = '/api/notifications/read-all';
  static String notificationRead(String id) =>
      '/api/notifications/$id/read';

  // ── Announcements ──
  // Backend: /api/announcements (src-tauri/src/http/announcements.rs)
  static const String announcementsActive = '/api/announcements/active';
  static const String announcementsRecent = '/api/announcements/recent';
  static String announcementDismiss(String id) =>
      '/api/announcements/$id/dismiss';

  // ── Customer registration ──
  static const String registerValidateInvite =
      '/api/public/customer-invite/validate';
  static const String registerAcceptInvite =
      '/api/public/customer-register';

  // ── Plans (public) ──
  static const String plans = '/api/plans';

  // ── Settings (public) ──
  static const String publicSettings = '/api/settings/public';

  // ── Payments (legacy aliases for payment_service) ──
  static const String myPayments = '/api/payment/invoices';
  static String paymentChannels(String invoiceId) =>
      '/api/payment/invoices/$invoiceId/payment-channels';
  static String payInvoice(String invoiceId) =>
      '/api/payment/invoices/$invoiceId/midtrans';
  static String paymentById(String transactionId) =>
      '/api/payment/invoices/$transactionId/status';

  // ── Network Status (portal) ──
  static const String networkStatus = '/api/customers/portal/network-status';

  // ── Realtime ──
  static const String wsRealtime = '/api/ws';

  // ── Work Orders (admin/technician) ──
  // Backend: /api/admin/work-orders (src-tauri/src/http/work_orders.rs)
  static const String workOrders = '/api/admin/work-orders';
  static String workOrderById(String id) => '/api/admin/work-orders/$id';
  static String workOrderClaim(String id) => '/api/admin/work-orders/$id/claim';
  static String workOrderStart(String id) => '/api/admin/work-orders/$id/start';
  static String workOrderComplete(String id) =>
      '/api/admin/work-orders/$id/complete';
  static String workOrderCancel(String id) =>
      '/api/admin/work-orders/$id/cancel';
  static String workOrderReopen(String id) =>
      '/api/admin/work-orders/$id/reopen';
  static String workOrderRescheduleRequest(String id) =>
      '/api/admin/work-orders/$id/reschedule-request';
  static String workOrderRescheduleApprove(String id) =>
      '/api/admin/work-orders/$id/reschedule-request/approve';
  static String workOrderRescheduleReject(String id) =>
      '/api/admin/work-orders/$id/reschedule-request/reject';

  /// Build URL with path parameter substitution.
  static String withParam(String endpoint, String name, String value) {
    return endpoint.replaceAll('{$name}', value);
  }
}
