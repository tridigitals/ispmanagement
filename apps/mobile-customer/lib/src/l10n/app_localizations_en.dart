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
}
