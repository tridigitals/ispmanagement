import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_id.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale) : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate = _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates = <LocalizationsDelegate<dynamic>>[
    delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
  ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('id')
  ];

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'ISP Teknisi'**
  String get appTitle;

  /// No description provided for @home.
  ///
  /// In en, this message translates to:
  /// **'Home'**
  String get home;

  /// No description provided for @profile.
  ///
  /// In en, this message translates to:
  /// **'Profile'**
  String get profile;

  /// No description provided for @settings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settings;

  /// No description provided for @support.
  ///
  /// In en, this message translates to:
  /// **'Support'**
  String get support;

  /// No description provided for @login.
  ///
  /// In en, this message translates to:
  /// **'Login'**
  String get login;

  /// No description provided for @logout.
  ///
  /// In en, this message translates to:
  /// **'Logout'**
  String get logout;

  /// No description provided for @register.
  ///
  /// In en, this message translates to:
  /// **'Register'**
  String get register;

  /// No description provided for @createAccount.
  ///
  /// In en, this message translates to:
  /// **'Create account'**
  String get createAccount;

  /// No description provided for @email.
  ///
  /// In en, this message translates to:
  /// **'Email'**
  String get email;

  /// No description provided for @phone.
  ///
  /// In en, this message translates to:
  /// **'Phone'**
  String get phone;

  /// No description provided for @password.
  ///
  /// In en, this message translates to:
  /// **'***'**
  String get password;

  /// No description provided for @currentPassword.
  ///
  /// In en, this message translates to:
  /// **'Current password'**
  String get currentPassword;

  /// No description provided for @newPassword.
  ///
  /// In en, this message translates to:
  /// **'New password'**
  String get newPassword;

  /// No description provided for @confirmNewPassword.
  ///
  /// In en, this message translates to:
  /// **'Confirm new password'**
  String get confirmNewPassword;

  /// No description provided for @confirmPassword.
  ///
  /// In en, this message translates to:
  /// **'Confirm password'**
  String get confirmPassword;

  /// No description provided for @fullName.
  ///
  /// In en, this message translates to:
  /// **'Full name'**
  String get fullName;

  /// No description provided for @forgotPassword.
  ///
  /// In en, this message translates to:
  /// **'Forgot password'**
  String get forgotPassword;

  /// No description provided for @forgotPasswordHeadline.
  ///
  /// In en, this message translates to:
  /// **'Forgot your password?'**
  String get forgotPasswordHeadline;

  /// No description provided for @forgotPasswordSub.
  ///
  /// In en, this message translates to:
  /// **'Enter your email and we\'ll send a reset link.'**
  String get forgotPasswordSub;

  /// No description provided for @sendResetLink.
  ///
  /// In en, this message translates to:
  /// **'Send reset link'**
  String get sendResetLink;

  /// No description provided for @reasonOptional.
  ///
  /// In en, this message translates to:
  /// **'Reason (optional)'**
  String get reasonOptional;

  /// No description provided for @reasonHint.
  ///
  /// In en, this message translates to:
  /// **'Tell us what happened...'**
  String get reasonHint;

  /// No description provided for @backToLogin.
  ///
  /// In en, this message translates to:
  /// **'Back to login'**
  String get backToLogin;

  /// No description provided for @back.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get back;

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @disable.
  ///
  /// In en, this message translates to:
  /// **'Disable'**
  String get disable;

  /// No description provided for @hiPrefix.
  ///
  /// In en, this message translates to:
  /// **'Hi'**
  String get hiPrefix;

  /// No description provided for @noSubscription.
  ///
  /// In en, this message translates to:
  /// **'No subscription yet'**
  String get noSubscription;

  /// No description provided for @noInvoices.
  ///
  /// In en, this message translates to:
  /// **'No invoices yet'**
  String get noInvoices;

  /// No description provided for @noNotifications.
  ///
  /// In en, this message translates to:
  /// **'No notifications yet'**
  String get noNotifications;

  /// No description provided for @recentInvoices.
  ///
  /// In en, this message translates to:
  /// **'Recent Invoices'**
  String get recentInvoices;

  /// No description provided for @seeAll.
  ///
  /// In en, this message translates to:
  /// **'See all'**
  String get seeAll;

  /// No description provided for @notifications.
  ///
  /// In en, this message translates to:
  /// **'Notifications'**
  String get notifications;

  /// No description provided for @notifInvoice.
  ///
  /// In en, this message translates to:
  /// **'Invoice reminders'**
  String get notifInvoice;

  /// No description provided for @notifInvoiceSub.
  ///
  /// In en, this message translates to:
  /// **'Notify me 3 days before due date and on due date'**
  String get notifInvoiceSub;

  /// No description provided for @notifOutage.
  ///
  /// In en, this message translates to:
  /// **'Network outages'**
  String get notifOutage;

  /// No description provided for @notifOutageSub.
  ///
  /// In en, this message translates to:
  /// **'Notify me about outages in my area'**
  String get notifOutageSub;

  /// No description provided for @notifPromo.
  ///
  /// In en, this message translates to:
  /// **'Promotions & offers'**
  String get notifPromo;

  /// No description provided for @notifPromoSub.
  ///
  /// In en, this message translates to:
  /// **'Get promo info from the ISP'**
  String get notifPromoSub;

  /// No description provided for @markAllRead.
  ///
  /// In en, this message translates to:
  /// **'Mark all as read'**
  String get markAllRead;

  /// No description provided for @contactUs.
  ///
  /// In en, this message translates to:
  /// **'Contact Us'**
  String get contactUs;

  /// No description provided for @faq.
  ///
  /// In en, this message translates to:
  /// **'FAQ'**
  String get faq;

  /// No description provided for @changePassword.
  ///
  /// In en, this message translates to:
  /// **'Change password'**
  String get changePassword;

  /// No description provided for @editProfile.
  ///
  /// In en, this message translates to:
  /// **'Edit profile'**
  String get editProfile;

  /// No description provided for @verifyOtp.
  ///
  /// In en, this message translates to:
  /// **'Verify OTP'**
  String get verifyOtp;

  /// No description provided for @verify2fa.
  ///
  /// In en, this message translates to:
  /// **'Verify 2FA'**
  String get verify2fa;

  /// No description provided for @verify2faHeadline.
  ///
  /// In en, this message translates to:
  /// **'Two-Factor Verification'**
  String get verify2faHeadline;

  /// No description provided for @verify.
  ///
  /// In en, this message translates to:
  /// **'Verify'**
  String get verify;

  /// No description provided for @loginWithOtp.
  ///
  /// In en, this message translates to:
  /// **'Login with OTP'**
  String get loginWithOtp;

  /// No description provided for @otpLoginHeadline.
  ///
  /// In en, this message translates to:
  /// **'Passwordless login'**
  String get otpLoginHeadline;

  /// No description provided for @otpLoginSub.
  ///
  /// In en, this message translates to:
  /// **'We\'ll send a 6-digit code to your phone.'**
  String get otpLoginSub;

  /// No description provided for @sendOtp.
  ///
  /// In en, this message translates to:
  /// **'Send OTP'**
  String get sendOtp;

  /// No description provided for @otpVerifyHeadline.
  ///
  /// In en, this message translates to:
  /// **'Enter verification code'**
  String get otpVerifyHeadline;

  /// No description provided for @otpVerifySub.
  ///
  /// In en, this message translates to:
  /// **'A code was sent to {phone}'**
  String otpVerifySub(String phone);

  /// No description provided for @otpSent.
  ///
  /// In en, this message translates to:
  /// **'OTP code sent'**
  String get otpSent;

  /// No description provided for @otpResent.
  ///
  /// In en, this message translates to:
  /// **'OTP code resent'**
  String get otpResent;

  /// No description provided for @resendOtp.
  ///
  /// In en, this message translates to:
  /// **'Resend code'**
  String get resendOtp;

  /// No description provided for @resendIn.
  ///
  /// In en, this message translates to:
  /// **'Resend in {seconds}s'**
  String resendIn(int seconds);

  /// No description provided for @otpCode.
  ///
  /// In en, this message translates to:
  /// **'OTP code'**
  String get otpCode;

  /// No description provided for @backupCode.
  ///
  /// In en, this message translates to:
  /// **'Backup code'**
  String get backupCode;

  /// No description provided for @useAuthenticator.
  ///
  /// In en, this message translates to:
  /// **'Use authenticator app'**
  String get useAuthenticator;

  /// No description provided for @useBackupCode.
  ///
  /// In en, this message translates to:
  /// **'Use backup code'**
  String get useBackupCode;

  /// No description provided for @twoFactorAuth.
  ///
  /// In en, this message translates to:
  /// **'Two-Factor Auth'**
  String get twoFactorAuth;

  /// No description provided for @twoFaOn.
  ///
  /// In en, this message translates to:
  /// **'On'**
  String get twoFaOn;

  /// No description provided for @twoFaOff.
  ///
  /// In en, this message translates to:
  /// **'Off'**
  String get twoFaOff;

  /// No description provided for @twoFaRequired.
  ///
  /// In en, this message translates to:
  /// **'Required by organization'**
  String get twoFaRequired;

  /// No description provided for @twoFaEnabled.
  ///
  /// In en, this message translates to:
  /// **'2FA enabled successfully'**
  String get twoFaEnabled;

  /// No description provided for @twoFaHeadline.
  ///
  /// In en, this message translates to:
  /// **'Secure your account'**
  String get twoFaHeadline;

  /// No description provided for @twoFaSub.
  ///
  /// In en, this message translates to:
  /// **'Scan this QR with Google Authenticator or Authy, then enter the 6-digit code.'**
  String get twoFaSub;

  /// No description provided for @enable2fa.
  ///
  /// In en, this message translates to:
  /// **'Enable 2FA'**
  String get enable2fa;

  /// No description provided for @confirmEnable.
  ///
  /// In en, this message translates to:
  /// **'Confirm'**
  String get confirmEnable;

  /// No description provided for @disable2faConfirmTitle.
  ///
  /// In en, this message translates to:
  /// **'Disable 2FA?'**
  String get disable2faConfirmTitle;

  /// No description provided for @disable2faConfirmBody.
  ///
  /// In en, this message translates to:
  /// **'Your account will be less secure. You can re-enable it later.'**
  String get disable2faConfirmBody;

  /// No description provided for @biometric.
  ///
  /// In en, this message translates to:
  /// **'Biometric login'**
  String get biometric;

  /// No description provided for @biometricSub.
  ///
  /// In en, this message translates to:
  /// **'Use fingerprint or Face ID to log in'**
  String get biometricSub;

  /// No description provided for @biometricNotAvailable.
  ///
  /// In en, this message translates to:
  /// **'Biometric is not available on this device'**
  String get biometricNotAvailable;

  /// No description provided for @biometricEnableReason.
  ///
  /// In en, this message translates to:
  /// **'Confirm to enable biometric login'**
  String get biometricEnableReason;

  /// No description provided for @passwordChanged.
  ///
  /// In en, this message translates to:
  /// **'Password changed'**
  String get passwordChanged;

  /// No description provided for @passwordRule.
  ///
  /// In en, this message translates to:
  /// **'Minimum 8 chars, must include a letter and a number'**
  String get passwordRule;

  /// No description provided for @passwordMismatch.
  ///
  /// In en, this message translates to:
  /// **'Passwords do not match'**
  String get passwordMismatch;

  /// No description provided for @profileUpdated.
  ///
  /// In en, this message translates to:
  /// **'Profile updated'**
  String get profileUpdated;

  /// No description provided for @inviteCode.
  ///
  /// In en, this message translates to:
  /// **'Invite code'**
  String get inviteCode;

  /// No description provided for @inviteValidateFirst.
  ///
  /// In en, this message translates to:
  /// **'Please validate your invite code first'**
  String get inviteValidateFirst;

  /// No description provided for @registerHeadline.
  ///
  /// In en, this message translates to:
  /// **'Activate your account'**
  String get registerHeadline;

  /// No description provided for @registerSub.
  ///
  /// In en, this message translates to:
  /// **'Enter the invite code from our email/WhatsApp'**
  String get registerSub;

  /// No description provided for @registerSuccess.
  ///
  /// In en, this message translates to:
  /// **'Account created, welcome!'**
  String get registerSuccess;

  /// No description provided for @account.
  ///
  /// In en, this message translates to:
  /// **'Account'**
  String get account;

  /// No description provided for @about.
  ///
  /// In en, this message translates to:
  /// **'About'**
  String get about;

  /// No description provided for @privacyPolicy.
  ///
  /// In en, this message translates to:
  /// **'Privacy Policy'**
  String get privacyPolicy;

  /// No description provided for @termsOfService.
  ///
  /// In en, this message translates to:
  /// **'Terms of Service'**
  String get termsOfService;

  /// No description provided for @myInvoices.
  ///
  /// In en, this message translates to:
  /// **'My Invoices'**
  String get myInvoices;

  /// No description provided for @mySubscriptions.
  ///
  /// In en, this message translates to:
  /// **'My Subscriptions'**
  String get mySubscriptions;

  /// No description provided for @invalidEmail.
  ///
  /// In en, this message translates to:
  /// **'Please enter a valid email address'**
  String get invalidEmail;

  /// No description provided for @passwordTooShort.
  ///
  /// In en, this message translates to:
  /// **'Password must be at least 8 characters'**
  String get passwordTooShort;

  /// No description provided for @enter2faCode.
  ///
  /// In en, this message translates to:
  /// **'Enter the 6-digit code from your authenticator app'**
  String get enter2faCode;

  /// No description provided for @officeAddress.
  ///
  /// In en, this message translates to:
  /// **'Office Address'**
  String get officeAddress;

  /// No description provided for @serviceHours.
  ///
  /// In en, this message translates to:
  /// **'Service Hours'**
  String get serviceHours;

  /// No description provided for @myTickets.
  ///
  /// In en, this message translates to:
  /// **'My Tickets'**
  String get myTickets;

  /// No description provided for @noTickets.
  ///
  /// In en, this message translates to:
  /// **'No tickets yet'**
  String get noTickets;

  /// No description provided for @createFirstTicket.
  ///
  /// In en, this message translates to:
  /// **'Create your first support ticket'**
  String get createFirstTicket;

  /// No description provided for @newTicket.
  ///
  /// In en, this message translates to:
  /// **'New Ticket'**
  String get newTicket;

  /// No description provided for @paymentInstruction.
  ///
  /// In en, this message translates to:
  /// **'Payment Instructions'**
  String get paymentInstruction;

  /// No description provided for @totalPayment.
  ///
  /// In en, this message translates to:
  /// **'Total Payment'**
  String get totalPayment;

  /// No description provided for @choosePaymentMethod.
  ///
  /// In en, this message translates to:
  /// **'Choose Payment Method'**
  String get choosePaymentMethod;

  /// No description provided for @changePasswordHeadline.
  ///
  /// In en, this message translates to:
  /// **'Change your password'**
  String get changePasswordHeadline;

  /// No description provided for @speedTest.
  ///
  /// In en, this message translates to:
  /// **'Speed Test'**
  String get speedTest;

  /// No description provided for @pay.
  ///
  /// In en, this message translates to:
  /// **'Pay'**
  String get pay;

  /// No description provided for @report.
  ///
  /// In en, this message translates to:
  /// **'Report'**
  String get report;

  /// No description provided for @share.
  ///
  /// In en, this message translates to:
  /// **'Share'**
  String get share;

  /// No description provided for @unpaidBills.
  ///
  /// In en, this message translates to:
  /// **'Unpaid Bills'**
  String get unpaidBills;

  /// No description provided for @noBills.
  ///
  /// In en, this message translates to:
  /// **'No bills'**
  String get noBills;

  /// No description provided for @activePlan.
  ///
  /// In en, this message translates to:
  /// **'Active Plan'**
  String get activePlan;

  /// No description provided for @fromTotalSubscriptions.
  ///
  /// In en, this message translates to:
  /// **'From total subscriptions'**
  String get fromTotalSubscriptions;

  /// No description provided for @internetPackage.
  ///
  /// In en, this message translates to:
  /// **'Internet Package'**
  String get internetPackage;

  /// No description provided for @quickActions.
  ///
  /// In en, this message translates to:
  /// **'Quick Actions'**
  String get quickActions;

  /// No description provided for @subscriptionDetail.
  ///
  /// In en, this message translates to:
  /// **'Subscription Details'**
  String get subscriptionDetail;

  /// No description provided for @connectionDetails.
  ///
  /// In en, this message translates to:
  /// **'Connection Details'**
  String get connectionDetails;

  /// No description provided for @billingInfo.
  ///
  /// In en, this message translates to:
  /// **'Billing Information'**
  String get billingInfo;

  /// No description provided for @router.
  ///
  /// In en, this message translates to:
  /// **'Router'**
  String get router;

  /// No description provided for @location.
  ///
  /// In en, this message translates to:
  /// **'Location'**
  String get location;

  /// No description provided for @notes.
  ///
  /// In en, this message translates to:
  /// **'Notes'**
  String get notes;

  /// No description provided for @price.
  ///
  /// In en, this message translates to:
  /// **'Price'**
  String get price;

  /// No description provided for @cycle.
  ///
  /// In en, this message translates to:
  /// **'Billing Cycle'**
  String get cycle;

  /// No description provided for @startsAt.
  ///
  /// In en, this message translates to:
  /// **'Start Date'**
  String get startsAt;

  /// No description provided for @endsAt.
  ///
  /// In en, this message translates to:
  /// **'End Date'**
  String get endsAt;

  /// No description provided for @gracePeriod.
  ///
  /// In en, this message translates to:
  /// **'Grace Period'**
  String get gracePeriod;

  /// No description provided for @reportOutage.
  ///
  /// In en, this message translates to:
  /// **'Report Outage'**
  String get reportOutage;

  /// No description provided for @retry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get retry;

  /// No description provided for @noPaymentUrl.
  ///
  /// In en, this message translates to:
  /// **'No payment URL available'**
  String get noPaymentUrl;

  /// No description provided for @noInvoicesYet.
  ///
  /// In en, this message translates to:
  /// **'No invoices yet'**
  String get noInvoicesYet;

  /// No description provided for @dueOn.
  ///
  /// In en, this message translates to:
  /// **'Due on'**
  String get dueOn;

  /// No description provided for @announcements.
  ///
  /// In en, this message translates to:
  /// **'Announcements'**
  String get announcements;

  /// No description provided for @announcementDetail.
  ///
  /// In en, this message translates to:
  /// **'Announcement Detail'**
  String get announcementDetail;

  /// No description provided for @noAnnouncements.
  ///
  /// In en, this message translates to:
  /// **'No announcements yet'**
  String get noAnnouncements;

  /// No description provided for @severity.
  ///
  /// In en, this message translates to:
  /// **'Severity'**
  String get severity;

  /// No description provided for @audience.
  ///
  /// In en, this message translates to:
  /// **'Audience'**
  String get audience;

  /// No description provided for @details.
  ///
  /// In en, this message translates to:
  /// **'Details'**
  String get details;

  /// No description provided for @darkMode.
  ///
  /// In en, this message translates to:
  /// **'Dark Mode'**
  String get darkMode;

  /// No description provided for @ticketStatusOpen.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get ticketStatusOpen;

  /// No description provided for @ticketStatusInProgress.
  ///
  /// In en, this message translates to:
  /// **'In Progress'**
  String get ticketStatusInProgress;

  /// No description provided for @ticketStatusWaitingCustomer.
  ///
  /// In en, this message translates to:
  /// **'Waiting Customer'**
  String get ticketStatusWaitingCustomer;

  /// No description provided for @ticketStatusWaitingStaff.
  ///
  /// In en, this message translates to:
  /// **'Waiting Staff'**
  String get ticketStatusWaitingStaff;

  /// No description provided for @ticketStatusResolved.
  ///
  /// In en, this message translates to:
  /// **'Resolved'**
  String get ticketStatusResolved;

  /// No description provided for @ticketStatusClosed.
  ///
  /// In en, this message translates to:
  /// **'Closed'**
  String get ticketStatusClosed;

  /// No description provided for @ticketStatusCancelled.
  ///
  /// In en, this message translates to:
  /// **'Cancelled'**
  String get ticketStatusCancelled;

  /// No description provided for @ticketPriorityLow.
  ///
  /// In en, this message translates to:
  /// **'Low'**
  String get ticketPriorityLow;

  /// No description provided for @ticketPriorityNormal.
  ///
  /// In en, this message translates to:
  /// **'Normal'**
  String get ticketPriorityNormal;

  /// No description provided for @ticketPriorityHigh.
  ///
  /// In en, this message translates to:
  /// **'High'**
  String get ticketPriorityHigh;

  /// No description provided for @ticketPriorityUrgent.
  ///
  /// In en, this message translates to:
  /// **'Urgent'**
  String get ticketPriorityUrgent;

  /// No description provided for @ticketCategoryGeneral.
  ///
  /// In en, this message translates to:
  /// **'General'**
  String get ticketCategoryGeneral;

  /// No description provided for @ticketCategoryBilling.
  ///
  /// In en, this message translates to:
  /// **'Billing'**
  String get ticketCategoryBilling;

  /// No description provided for @ticketCategoryTechnical.
  ///
  /// In en, this message translates to:
  /// **'Technical'**
  String get ticketCategoryTechnical;

  /// No description provided for @ticketCategoryInstallation.
  ///
  /// In en, this message translates to:
  /// **'Installation'**
  String get ticketCategoryInstallation;

  /// No description provided for @ticketActionCamera.
  ///
  /// In en, this message translates to:
  /// **'Take Photo'**
  String get ticketActionCamera;

  /// No description provided for @ticketActionFile.
  ///
  /// In en, this message translates to:
  /// **'Choose File'**
  String get ticketActionFile;

  /// No description provided for @ticketActionCameraSub.
  ///
  /// In en, this message translates to:
  /// **'Camera — requires camera access permission'**
  String get ticketActionCameraSub;

  /// No description provided for @ticketActionFileSub.
  ///
  /// In en, this message translates to:
  /// **'PDF, images, documents — from device storage'**
  String get ticketActionFileSub;

  /// No description provided for @ticketErrorCameraFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to open camera: {error}'**
  String ticketErrorCameraFailed(Object error);

  /// No description provided for @ticketErrorFileFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to pick file: {error}'**
  String ticketErrorFileFailed(Object error);

  /// No description provided for @ticketErrorSendFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to send: {error}'**
  String ticketErrorSendFailed(Object error);

  /// No description provided for @ticketErrorReplyFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to reply: {error}'**
  String ticketErrorReplyFailed(Object error);

  /// No description provided for @ticketErrorLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to load ticket'**
  String get ticketErrorLoadFailed;

  /// No description provided for @ticketErrorSessionExpired.
  ///
  /// In en, this message translates to:
  /// **'Session expired, please log in again'**
  String get ticketErrorSessionExpired;

  /// No description provided for @ticketFieldSubject.
  ///
  /// In en, this message translates to:
  /// **'Subject'**
  String get ticketFieldSubject;

  /// No description provided for @ticketFieldSubjectHint.
  ///
  /// In en, this message translates to:
  /// **'Brief summary'**
  String get ticketFieldSubjectHint;

  /// No description provided for @ticketFieldDescription.
  ///
  /// In en, this message translates to:
  /// **'Description'**
  String get ticketFieldDescription;

  /// No description provided for @ticketFieldDescriptionHint.
  ///
  /// In en, this message translates to:
  /// **'Describe the issue...'**
  String get ticketFieldDescriptionHint;

  /// No description provided for @ticketFieldReply.
  ///
  /// In en, this message translates to:
  /// **'Type a message...'**
  String get ticketFieldReply;

  /// No description provided for @ticketFieldAttachments.
  ///
  /// In en, this message translates to:
  /// **'Attachments'**
  String get ticketFieldAttachments;

  /// No description provided for @ticketFieldSubscription.
  ///
  /// In en, this message translates to:
  /// **'Related Subscription (optional)'**
  String get ticketFieldSubscription;

  /// No description provided for @ticketFieldNoSubscription.
  ///
  /// In en, this message translates to:
  /// **'Not linked'**
  String get ticketFieldNoSubscription;

  /// No description provided for @ticketValidationSubjectShort.
  ///
  /// In en, this message translates to:
  /// **'Subject must be at least 3 characters'**
  String get ticketValidationSubjectShort;

  /// No description provided for @ticketValidationDescriptionShort.
  ///
  /// In en, this message translates to:
  /// **'Description must be at least 10 characters'**
  String get ticketValidationDescriptionShort;

  /// No description provided for @ticketButtonAdd.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get ticketButtonAdd;

  /// No description provided for @ticketButtonSend.
  ///
  /// In en, this message translates to:
  /// **'Send Ticket'**
  String get ticketButtonSend;

  /// No description provided for @ticketButtonSending.
  ///
  /// In en, this message translates to:
  /// **'Sending...'**
  String get ticketButtonSending;

  /// No description provided for @ticketButtonSubmitReply.
  ///
  /// In en, this message translates to:
  /// **'Send Reply'**
  String get ticketButtonSubmitReply;

  /// No description provided for @ticketButtonSendingReply.
  ///
  /// In en, this message translates to:
  /// **'Sending...'**
  String get ticketButtonSendingReply;

  /// No description provided for @ticketButtonAttach.
  ///
  /// In en, this message translates to:
  /// **'Attach'**
  String get ticketButtonAttach;

  /// No description provided for @ticketButtonClose.
  ///
  /// In en, this message translates to:
  /// **'Close Ticket'**
  String get ticketButtonClose;

  /// No description provided for @ticketButtonReopen.
  ///
  /// In en, this message translates to:
  /// **'Reopen'**
  String get ticketButtonReopen;

  /// No description provided for @ticketButtonAssign.
  ///
  /// In en, this message translates to:
  /// **'Assign'**
  String get ticketButtonAssign;

  /// No description provided for @ticketButtonEscalate.
  ///
  /// In en, this message translates to:
  /// **'Escalate'**
  String get ticketButtonEscalate;

  /// No description provided for @ticketToastCreated.
  ///
  /// In en, this message translates to:
  /// **'Ticket sent — our team will follow up'**
  String get ticketToastCreated;

  /// No description provided for @ticketToastReplySent.
  ///
  /// In en, this message translates to:
  /// **'Reply sent'**
  String get ticketToastReplySent;

  /// No description provided for @ticketToastClosed.
  ///
  /// In en, this message translates to:
  /// **'Ticket closed'**
  String get ticketToastClosed;

  /// No description provided for @ticketToastReopened.
  ///
  /// In en, this message translates to:
  /// **'Ticket reopened'**
  String get ticketToastReopened;

  /// No description provided for @ticketQuickActionNoInternet.
  ///
  /// In en, this message translates to:
  /// **'No Internet'**
  String get ticketQuickActionNoInternet;

  /// No description provided for @ticketQuickActionNoInternetSubject.
  ///
  /// In en, this message translates to:
  /// **'Cannot access the internet'**
  String get ticketQuickActionNoInternetSubject;

  /// No description provided for @ticketQuickActionNoInternetDesc.
  ///
  /// In en, this message translates to:
  /// **'My internet connection is unavailable. Please check.'**
  String get ticketQuickActionNoInternetDesc;

  /// No description provided for @ticketQuickActionSlow.
  ///
  /// In en, this message translates to:
  /// **'Slow WiFi'**
  String get ticketQuickActionSlow;

  /// No description provided for @ticketQuickActionSlowSubject.
  ///
  /// In en, this message translates to:
  /// **'WiFi slow / frequently disconnects'**
  String get ticketQuickActionSlowSubject;

  /// No description provided for @ticketQuickActionSlowDesc.
  ///
  /// In en, this message translates to:
  /// **'WiFi feels slow or unstable. Please check.'**
  String get ticketQuickActionSlowDesc;

  /// No description provided for @ticketQuickActionOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get ticketQuickActionOther;

  /// No description provided for @ticketAuthorYou.
  ///
  /// In en, this message translates to:
  /// **'You'**
  String get ticketAuthorYou;

  /// No description provided for @ticketAuthorSupport.
  ///
  /// In en, this message translates to:
  /// **'Support'**
  String get ticketAuthorSupport;

  /// No description provided for @ticketAuthorCustomer.
  ///
  /// In en, this message translates to:
  /// **'Customer'**
  String get ticketAuthorCustomer;

  /// No description provided for @ticketAuthorStaff.
  ///
  /// In en, this message translates to:
  /// **'Staff'**
  String get ticketAuthorStaff;

  /// No description provided for @ticketAuthorAnonymous.
  ///
  /// In en, this message translates to:
  /// **'Anonymous'**
  String get ticketAuthorAnonymous;

  /// No description provided for @ticketViewSubscription.
  ///
  /// In en, this message translates to:
  /// **'View related subscription'**
  String get ticketViewSubscription;

  /// No description provided for @ticketConversation.
  ///
  /// In en, this message translates to:
  /// **'Conversation'**
  String get ticketConversation;

  /// No description provided for @ticketNoMessages.
  ///
  /// In en, this message translates to:
  /// **'No messages yet'**
  String get ticketNoMessages;

  /// No description provided for @ticketNoMessagesHint.
  ///
  /// In en, this message translates to:
  /// **'Send your first message to start the conversation'**
  String get ticketNoMessagesHint;

  /// No description provided for @ticketClosedNotice.
  ///
  /// In en, this message translates to:
  /// **'This ticket is closed. You cannot reply.'**
  String get ticketClosedNotice;

  /// No description provided for @ticketAssignee.
  ///
  /// In en, this message translates to:
  /// **'Assignee'**
  String get ticketAssignee;

  /// No description provided for @ticketUnassigned.
  ///
  /// In en, this message translates to:
  /// **'Unassigned'**
  String get ticketUnassigned;

  /// No description provided for @ticketSatisfaction.
  ///
  /// In en, this message translates to:
  /// **'Satisfaction'**
  String get ticketSatisfaction;

  /// No description provided for @ticketRateHint.
  ///
  /// In en, this message translates to:
  /// **'How satisfied are you?'**
  String get ticketRateHint;

  /// No description provided for @ticketCommentOptional.
  ///
  /// In en, this message translates to:
  /// **'Comment (optional)'**
  String get ticketCommentOptional;

  /// No description provided for @ticketSubmitRating.
  ///
  /// In en, this message translates to:
  /// **'Submit Rating'**
  String get ticketSubmitRating;

  /// No description provided for @ticketAdminListTitle.
  ///
  /// In en, this message translates to:
  /// **'Support Tickets'**
  String get ticketAdminListTitle;

  /// No description provided for @ticketAdminTabAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get ticketAdminTabAll;

  /// No description provided for @ticketAdminTabOpen.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get ticketAdminTabOpen;

  /// No description provided for @ticketAdminTabInProgress.
  ///
  /// In en, this message translates to:
  /// **'In Progress'**
  String get ticketAdminTabInProgress;

  /// No description provided for @ticketAdminTabClosed.
  ///
  /// In en, this message translates to:
  /// **'Closed'**
  String get ticketAdminTabClosed;

  /// No description provided for @ticketAdminEmpty.
  ///
  /// In en, this message translates to:
  /// **'No tickets yet'**
  String get ticketAdminEmpty;

  /// No description provided for @ticketAdminFilterAll.
  ///
  /// In en, this message translates to:
  /// **'All categories'**
  String get ticketAdminFilterAll;

  /// No description provided for @ticketAdminFilterOpen.
  ///
  /// In en, this message translates to:
  /// **'Open only'**
  String get ticketAdminFilterOpen;

  /// Success message after disabling 2FA
  ///
  /// In en, this message translates to:
  /// **'2FA has been disabled'**
  String get twoFaDisabledSuccess;

  /// Title for OTP verification dialog
  ///
  /// In en, this message translates to:
  /// **'Enter Verification Code'**
  String get enterVerificationCode;

  /// Subtext in OTP verification dialog
  ///
  /// In en, this message translates to:
  /// **'OTP code has been sent to your email'**
  String get otpSentToEmail;

  /// Label for OTP input field
  ///
  /// In en, this message translates to:
  /// **'Verification Code'**
  String get verificationCode;

  /// No description provided for @tickets.
  ///
  /// In en, this message translates to:
  /// **'Tickets'**
  String get tickets;

  /// No description provided for @ticketStatsAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get ticketStatsAll;

  /// No description provided for @ticketStatsOpen.
  ///
  /// In en, this message translates to:
  /// **'Open'**
  String get ticketStatsOpen;

  /// No description provided for @ticketStatsPending.
  ///
  /// In en, this message translates to:
  /// **'Pending'**
  String get ticketStatsPending;

  /// No description provided for @ticketStatsClosed.
  ///
  /// In en, this message translates to:
  /// **'Closed'**
  String get ticketStatsClosed;

  /// No description provided for @recentTickets.
  ///
  /// In en, this message translates to:
  /// **'Recent Tickets'**
  String get recentTickets;

  /// No description provided for @noAssignedTickets.
  ///
  /// In en, this message translates to:
  /// **'No assigned tickets'**
  String get noAssignedTickets;

  /// No description provided for @assignedToYou.
  ///
  /// In en, this message translates to:
  /// **'Assigned to you'**
  String get assignedToYou;

  /// No description provided for @workOrders.
  ///
  /// In en, this message translates to:
  /// **'Work Orders'**
  String get workOrders;

  /// No description provided for @workOrderDetail.
  ///
  /// In en, this message translates to:
  /// **'Work Order Detail'**
  String get workOrderDetail;

  /// No description provided for @workOrderClaim.
  ///
  /// In en, this message translates to:
  /// **'Claim'**
  String get workOrderClaim;

  /// No description provided for @workOrderStart.
  ///
  /// In en, this message translates to:
  /// **'Start Work'**
  String get workOrderStart;

  /// No description provided for @workOrderComplete.
  ///
  /// In en, this message translates to:
  /// **'Complete'**
  String get workOrderComplete;

  /// No description provided for @workOrderCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get workOrderCancel;

  /// No description provided for @workOrderReopen.
  ///
  /// In en, this message translates to:
  /// **'Reopen'**
  String get workOrderReopen;

  /// No description provided for @workOrderNotes.
  ///
  /// In en, this message translates to:
  /// **'Notes'**
  String get workOrderNotes;

  /// No description provided for @workOrderNotesHint.
  ///
  /// In en, this message translates to:
  /// **'Installation notes, issues found, etc.'**
  String get workOrderNotesHint;

  /// No description provided for @workOrderStatusPending.
  ///
  /// In en, this message translates to:
  /// **'Pending'**
  String get workOrderStatusPending;

  /// No description provided for @workOrderStatusAssigned.
  ///
  /// In en, this message translates to:
  /// **'Assigned'**
  String get workOrderStatusAssigned;

  /// No description provided for @workOrderStatusInProgress.
  ///
  /// In en, this message translates to:
  /// **'In Progress'**
  String get workOrderStatusInProgress;

  /// No description provided for @workOrderStatusCompleted.
  ///
  /// In en, this message translates to:
  /// **'Completed'**
  String get workOrderStatusCompleted;

  /// No description provided for @workOrderStatusCancelled.
  ///
  /// In en, this message translates to:
  /// **'Cancelled'**
  String get workOrderStatusCancelled;

  /// No description provided for @workOrderNoAssigned.
  ///
  /// In en, this message translates to:
  /// **'No work orders assigned'**
  String get workOrderNoAssigned;

  /// No description provided for @workOrderTabAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get workOrderTabAll;

  /// No description provided for @workOrderTabPending.
  ///
  /// In en, this message translates to:
  /// **'Pending'**
  String get workOrderTabPending;

  /// No description provided for @workOrderTabInProgress.
  ///
  /// In en, this message translates to:
  /// **'In Progress'**
  String get workOrderTabInProgress;

  /// No description provided for @workOrderTabCompleted.
  ///
  /// In en, this message translates to:
  /// **'Completed'**
  String get workOrderTabCompleted;

  /// No description provided for @workOrderConfirmed.
  ///
  /// In en, this message translates to:
  /// **'Work order completed'**
  String get workOrderConfirmed;

  /// No description provided for @workOrderClaimed.
  ///
  /// In en, this message translates to:
  /// **'Work order claimed'**
  String get workOrderClaimed;

  /// No description provided for @workOrderStarted.
  ///
  /// In en, this message translates to:
  /// **'Work order started'**
  String get workOrderStarted;

  /// No description provided for @workOrderCancelled.
  ///
  /// In en, this message translates to:
  /// **'Work order cancelled'**
  String get workOrderCancelled;

  /// No description provided for @workOrderErrorLoad.
  ///
  /// In en, this message translates to:
  /// **'Failed to load work order'**
  String get workOrderErrorLoad;

  /// No description provided for @workOrderPackage.
  ///
  /// In en, this message translates to:
  /// **'Package'**
  String get workOrderPackage;

  /// No description provided for @workOrderCustomer.
  ///
  /// In en, this message translates to:
  /// **'Customer'**
  String get workOrderCustomer;

  /// No description provided for @workOrderSchedule.
  ///
  /// In en, this message translates to:
  /// **'Schedule'**
  String get workOrderSchedule;

  /// No description provided for @workOrderRouter.
  ///
  /// In en, this message translates to:
  /// **'Router'**
  String get workOrderRouter;

  /// No description provided for @workOrderLocation.
  ///
  /// In en, this message translates to:
  /// **'Location'**
  String get workOrderLocation;

  /// No description provided for @workOrderStepClaim.
  ///
  /// In en, this message translates to:
  /// **'Claim'**
  String get workOrderStepClaim;

  /// No description provided for @workOrderStepStart.
  ///
  /// In en, this message translates to:
  /// **'Start'**
  String get workOrderStepStart;

  /// No description provided for @workOrderStepComplete.
  ///
  /// In en, this message translates to:
  /// **'Complete'**
  String get workOrderStepComplete;

  /// No description provided for @workOrderStepDone.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get workOrderStepDone;

  /// No description provided for @workOrderPhone.
  ///
  /// In en, this message translates to:
  /// **'Call'**
  String get workOrderPhone;

  /// No description provided for @workOrderWhatsApp.
  ///
  /// In en, this message translates to:
  /// **'WA'**
  String get workOrderWhatsApp;

  /// No description provided for @workOrderMaps.
  ///
  /// In en, this message translates to:
  /// **'Maps'**
  String get workOrderMaps;

  /// No description provided for @noPhoneNumber.
  ///
  /// In en, this message translates to:
  /// **'No phone number'**
  String get noPhoneNumber;
}

class _AppLocalizationsDelegate extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) => <String>['en', 'id'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {


  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en': return AppLocalizationsEn();
    case 'id': return AppLocalizationsId();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.'
  );
}
