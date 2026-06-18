import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'ISP Customer';

  @override
  String get home => 'Home';

  @override
  String get profile => 'Profile';

  @override
  String get settings => 'Settings';

  @override
  String get support => 'Support';

  @override
  String get login => 'Login';

  @override
  String get logout => 'Logout';

  @override
  String get register => 'Register';

  @override
  String get createAccount => 'Create account';

  @override
  String get email => 'Email';

  @override
  String get phone => 'Phone';

  @override
  String get password => '***';

  @override
  String get currentPassword => 'Current password';

  @override
  String get newPassword => 'New password';

  @override
  String get confirmNewPassword => 'Confirm new password';

  @override
  String get confirmPassword => 'Confirm password';

  @override
  String get fullName => 'Full name';

  @override
  String get forgotPassword => 'Forgot password';

  @override
  String get forgotPasswordHeadline => 'Forgot your password?';

  @override
  String get forgotPasswordSub => 'Enter your email and we\'ll send a reset link.';

  @override
  String get sendResetLink => 'Send reset link';

  @override
  String get reasonOptional => 'Reason (optional)';

  @override
  String get reasonHint => 'Tell us what happened...';

  @override
  String get backToLogin => 'Back to login';

  @override
  String get back => 'Back';

  @override
  String get save => 'Save';

  @override
  String get cancel => 'Cancel';

  @override
  String get disable => 'Disable';

  @override
  String get hiPrefix => 'Hi';

  @override
  String get noSubscription => 'No subscription yet';

  @override
  String get noInvoices => 'No invoices yet';

  @override
  String get noNotifications => 'No notifications yet';

  @override
  String get recentInvoices => 'Recent Invoices';

  @override
  String get seeAll => 'See all';

  @override
  String get notifications => 'Notifications';

  @override
  String get notifInvoice => 'Invoice reminders';

  @override
  String get notifInvoiceSub => 'Notify me 3 days before due date and on due date';

  @override
  String get notifOutage => 'Network outages';

  @override
  String get notifOutageSub => 'Notify me about outages in my area';

  @override
  String get notifPromo => 'Promotions & offers';

  @override
  String get notifPromoSub => 'Get promo info from the ISP';

  @override
  String get markAllRead => 'Mark all as read';

  @override
  String get contactUs => 'Contact Us';

  @override
  String get faq => 'FAQ';

  @override
  String get changePassword => 'Change password';

  @override
  String get editProfile => 'Edit profile';

  @override
  String get verifyOtp => 'Verify OTP';

  @override
  String get verify2fa => 'Verify 2FA';

  @override
  String get verify2faHeadline => 'Two-Factor Verification';

  @override
  String get verify => 'Verify';

  @override
  String get loginWithOtp => 'Login with OTP';

  @override
  String get otpLoginHeadline => 'Passwordless login';

  @override
  String get otpLoginSub => 'We\'ll send a 6-digit code to your phone.';

  @override
  String get sendOtp => 'Send OTP';

  @override
  String get otpVerifyHeadline => 'Enter verification code';

  @override
  String otpVerifySub(String phone) {
    return 'A code was sent to $phone';
  }

  @override
  String get otpSent => 'OTP code sent';

  @override
  String get otpResent => 'OTP code resent';

  @override
  String get resendOtp => 'Resend code';

  @override
  String resendIn(int seconds) {
    return 'Resend in ${seconds}s';
  }

  @override
  String get otpCode => 'OTP code';

  @override
  String get backupCode => 'Backup code';

  @override
  String get useAuthenticator => 'Use authenticator app';

  @override
  String get useBackupCode => 'Use backup code';

  @override
  String get twoFactorAuth => 'Two-Factor Auth';

  @override
  String get twoFaOn => 'On';

  @override
  String get twoFaOff => 'Off';

  @override
  String get twoFaRequired => 'Required by organization';

  @override
  String get twoFaEnabled => '2FA enabled successfully';

  @override
  String get twoFaHeadline => 'Secure your account';

  @override
  String get twoFaSub => 'Scan this QR with Google Authenticator or Authy, then enter the 6-digit code.';

  @override
  String get enable2fa => 'Enable 2FA';

  @override
  String get confirmEnable => 'Confirm';

  @override
  String get disable2faConfirmTitle => 'Disable 2FA?';

  @override
  String get disable2faConfirmBody => 'Your account will be less secure. You can re-enable it later.';

  @override
  String get biometric => 'Biometric login';

  @override
  String get biometricSub => 'Use fingerprint or Face ID to log in';

  @override
  String get biometricNotAvailable => 'Biometric is not available on this device';

  @override
  String get biometricEnableReason => 'Confirm to enable biometric login';

  @override
  String get passwordChanged => 'Password changed';

  @override
  String get passwordRule => 'Minimum 8 chars, must include a letter and a number';

  @override
  String get passwordMismatch => 'Passwords do not match';

  @override
  String get profileUpdated => 'Profile updated';

  @override
  String get inviteCode => 'Invite code';

  @override
  String get inviteValidateFirst => 'Please validate your invite code first';

  @override
  String get registerHeadline => 'Activate your account';

  @override
  String get registerSub => 'Enter the invite code from our email/WhatsApp';

  @override
  String get registerSuccess => 'Account created, welcome!';

  @override
  String get account => 'Account';

  @override
  String get about => 'About';

  @override
  String get privacyPolicy => 'Privacy Policy';

  @override
  String get termsOfService => 'Terms of Service';

  @override
  String get myInvoices => 'My Invoices';

  @override
  String get mySubscriptions => 'My Subscriptions';

  @override
  String get invalidEmail => 'Please enter a valid email address';

  @override
  String get passwordTooShort => 'Password must be at least 8 characters';

  @override
  String get enter2faCode => 'Enter the 6-digit code from your authenticator app';

  @override
  String get officeAddress => 'Office Address';

  @override
  String get serviceHours => 'Service Hours';

  @override
  String get myTickets => 'My Tickets';

  @override
  String get noTickets => 'No tickets yet';

  @override
  String get createFirstTicket => 'Create your first support ticket';

  @override
  String get newTicket => 'New Ticket';

  @override
  String get paymentInstruction => 'Payment Instructions';

  @override
  String get totalPayment => 'Total Payment';

  @override
  String get choosePaymentMethod => 'Choose Payment Method';

  @override
  String get changePasswordHeadline => 'Change your password';

  @override
  String get speedTest => 'Speed Test';

  @override
  String get pay => 'Pay';

  @override
  String get report => 'Report';

  @override
  String get share => 'Share';

  @override
  String get unpaidBills => 'Unpaid Bills';

  @override
  String get noBills => 'No bills';

  @override
  String get activePlan => 'Active Plan';

  @override
  String get fromTotalSubscriptions => 'From total subscriptions';

  @override
  String get internetPackage => 'Internet Package';

  @override
  String get quickActions => 'Quick Actions';

  @override
  String get subscriptionDetail => 'Subscription Details';

  @override
  String get connectionDetails => 'Connection Details';

  @override
  String get billingInfo => 'Billing Information';

  @override
  String get router => 'Router';

  @override
  String get location => 'Location';

  @override
  String get notes => 'Notes';

  @override
  String get price => 'Price';

  @override
  String get cycle => 'Billing Cycle';

  @override
  String get startsAt => 'Start Date';

  @override
  String get endsAt => 'End Date';

  @override
  String get gracePeriod => 'Grace Period';

  @override
  String get reportOutage => 'Report Outage';

  @override
  String get retry => 'Retry';

  @override
  String get noPaymentUrl => 'No payment URL available';

  @override
  String get noInvoicesYet => 'No invoices yet';

  @override
  String get dueOn => 'Due on';

  @override
  String get announcements => 'Announcements';

  @override
  String get announcementDetail => 'Announcement Detail';

  @override
  String get noAnnouncements => 'No announcements yet';

  @override
  String get severity => 'Severity';

  @override
  String get audience => 'Audience';

  @override
  String get details => 'Details';

  @override
  String get darkMode => 'Dark Mode';

  @override
  String get ticketStatusOpen => 'Open';

  @override
  String get ticketStatusInProgress => 'In Progress';

  @override
  String get ticketStatusWaitingCustomer => 'Waiting Customer';

  @override
  String get ticketStatusWaitingStaff => 'Waiting Staff';

  @override
  String get ticketStatusResolved => 'Resolved';

  @override
  String get ticketStatusClosed => 'Closed';

  @override
  String get ticketStatusCancelled => 'Cancelled';

  @override
  String get ticketPriorityLow => 'Low';

  @override
  String get ticketPriorityNormal => 'Normal';

  @override
  String get ticketPriorityHigh => 'High';

  @override
  String get ticketPriorityUrgent => 'Urgent';

  @override
  String get ticketCategoryGeneral => 'General';

  @override
  String get ticketCategoryBilling => 'Billing';

  @override
  String get ticketCategoryTechnical => 'Technical';

  @override
  String get ticketCategoryInstallation => 'Installation';

  @override
  String get ticketActionCamera => 'Take Photo';

  @override
  String get ticketActionFile => 'Choose File';

  @override
  String get ticketActionCameraSub => 'Camera — requires camera access permission';

  @override
  String get ticketActionFileSub => 'PDF, images, documents — from device storage';

  @override
  String ticketErrorCameraFailed(Object error) {
    return 'Failed to open camera: $error';
  }

  @override
  String ticketErrorFileFailed(Object error) {
    return 'Failed to pick file: $error';
  }

  @override
  String ticketErrorSendFailed(Object error) {
    return 'Failed to send: $error';
  }

  @override
  String ticketErrorReplyFailed(Object error) {
    return 'Failed to reply: $error';
  }

  @override
  String get ticketErrorLoadFailed => 'Failed to load ticket';

  @override
  String get ticketFieldSubject => 'Subject';

  @override
  String get ticketFieldSubjectHint => 'Brief summary';

  @override
  String get ticketFieldDescription => 'Description';

  @override
  String get ticketFieldDescriptionHint => 'Describe the issue...';

  @override
  String get ticketFieldReply => 'Type a message...';

  @override
  String get ticketFieldAttachments => 'Attachments';

  @override
  String get ticketFieldSubscription => 'Related Subscription (optional)';

  @override
  String get ticketFieldNoSubscription => 'Not linked';

  @override
  String get ticketValidationSubjectShort => 'Subject must be at least 3 characters';

  @override
  String get ticketValidationDescriptionShort => 'Description must be at least 10 characters';

  @override
  String get ticketButtonAdd => 'Add';

  @override
  String get ticketButtonSend => 'Send Ticket';

  @override
  String get ticketButtonSending => 'Sending...';

  @override
  String get ticketButtonSubmitReply => 'Send Reply';

  @override
  String get ticketButtonSendingReply => 'Sending...';

  @override
  String get ticketButtonAttach => 'Attach';

  @override
  String get ticketButtonClose => 'Close Ticket';

  @override
  String get ticketButtonReopen => 'Reopen';

  @override
  String get ticketButtonAssign => 'Assign';

  @override
  String get ticketButtonEscalate => 'Escalate';

  @override
  String get ticketToastCreated => 'Ticket sent — our team will follow up';

  @override
  String get ticketToastReplySent => 'Reply sent';

  @override
  String get ticketToastClosed => 'Ticket closed';

  @override
  String get ticketToastReopened => 'Ticket reopened';

  @override
  String get ticketQuickActionNoInternet => 'No Internet';

  @override
  String get ticketQuickActionNoInternetSubject => 'Cannot access the internet';

  @override
  String get ticketQuickActionNoInternetDesc => 'My internet connection is unavailable. Please check.';

  @override
  String get ticketQuickActionSlow => 'Slow WiFi';

  @override
  String get ticketQuickActionSlowSubject => 'WiFi slow / frequently disconnects';

  @override
  String get ticketQuickActionSlowDesc => 'WiFi feels slow or unstable. Please check.';

  @override
  String get ticketQuickActionOther => 'Other';

  @override
  String get ticketAuthorYou => 'You';

  @override
  String get ticketAuthorSupport => 'Support';

  @override
  String get ticketAuthorCustomer => 'Customer';

  @override
  String get ticketAuthorStaff => 'Staff';

  @override
  String get ticketAuthorAnonymous => 'Anonymous';

  @override
  String get ticketViewSubscription => 'View related subscription';

  @override
  String get ticketConversation => 'Conversation';

  @override
  String get ticketNoMessages => 'No messages yet';

  @override
  String get ticketNoMessagesHint => 'Send your first message to start the conversation';

  @override
  String get ticketClosedNotice => 'This ticket is closed. You cannot reply.';

  @override
  String get ticketAssignee => 'Assignee';

  @override
  String get ticketUnassigned => 'Unassigned';

  @override
  String get ticketSatisfaction => 'Satisfaction';

  @override
  String get ticketRateHint => 'How satisfied are you?';

  @override
  String get ticketCommentOptional => 'Comment (optional)';

  @override
  String get ticketSubmitRating => 'Submit Rating';

  @override
  String get ticketAdminListTitle => 'Support Tickets';

  @override
  String get ticketAdminTabAll => 'All';

  @override
  String get ticketAdminTabOpen => 'Open';

  @override
  String get ticketAdminTabInProgress => 'In Progress';

  @override
  String get ticketAdminTabClosed => 'Closed';

  @override
  String get ticketAdminEmpty => 'No tickets yet';

  @override
  String get ticketAdminFilterAll => 'All categories';

  @override
  String get ticketAdminFilterOpen => 'Open only';
}
