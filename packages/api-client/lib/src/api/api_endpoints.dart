/// API endpoints constants for the ISP Management backend.
class ApiEndpoints {
  ApiEndpoints._();

  // Auth
  static const String authLogin = '/api/auth/login';
  static const String authRegister = '/api/auth/register';
  static const String authLogout = '/api/auth/logout';
  static const String authMe = '/api/auth/me';
  static const String authValidate = '/api/auth/validate';
  static const String authRefresh = '/api/auth/refresh';
  static const String authForgotPassword = '/api/auth/forgot-password';
  static const String authResetPassword = '/api/auth/reset-password';
  static const String authChangePassword = '/api/auth/change-password';
  static const String auth2faVerify = '/api/auth/2fa/verify';
  static const String auth2faEnable = '/api/auth/2fa/enable';

  // Portal
  static const String me = '/api/portal/me';
  static const String mySubscriptions = '/api/portal/subscriptions';
  static String mySubscriptionById(String id) =>
      '/api/portal/subscriptions/$id';
  static const String myInvoices = '/api/portal/invoices';
  static String myInvoiceById(String id) => '/api/portal/invoices/$id';
  static const String myTickets = '/api/portal/tickets';
  static String myTicketById(String id) => '/api/portal/tickets/$id';
  static String ticketMessages(String id) =>
      '/api/portal/tickets/$id/messages';
  static const String createTicket = '/api/portal/tickets';

  // Payments
  static String paymentChannels(String invoiceId) =>
      '/api/portal/invoices/$invoiceId/payment-channels';
  static String payInvoice(String invoiceId) =>
      '/api/portal/invoices/$invoiceId/pay';
  static String paymentById(String transactionId) =>
      '/api/portal/payments/$transactionId';

  // Notifications
  static const String notifications = '/api/portal/notifications';
  static const String notificationsUnreadCount =
      '/api/portal/notifications/unread-count';
  static const String notificationsReadAll =
      '/api/portal/notifications/read-all';
  static String notificationRead(String id) =>
      '/api/portal/notifications/$id/read';

  // Network
  static const String networkStatus = '/api/portal/network-status';
  static String mySubscriptionTraffic(String id) =>
      '/api/portal/subscriptions/$id/traffic';

  // Customer registration
  static const String registerValidateInvite =
      '/api/customer/registration/invites/validate';
  static const String registerAcceptInvite =
      '/api/customer/registration/invites/accept';

  // Profile
  static const String profile = '/api/portal/profile';
  static const String uploadAvatar = '/api/portal/profile/avatar';

  // Realtime
  static const String wsRealtime = '/ws';

  /// Build URL with path parameter substitution.
  static String withParam(String endpoint, String name, String value) {
    return endpoint.replaceAll('{$name}', value);
  }
}
