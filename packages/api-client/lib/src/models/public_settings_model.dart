/// Bank account info for manual bank transfer payments.
class BankAccountModel {
  const BankAccountModel({
    required this.id,
    required this.bankName,
    required this.accountNumber,
    required this.accountHolder,
    this.isActive = true,
  });

  final String id;
  final String bankName;
  final String accountNumber;
  final String accountHolder;
  final bool isActive;

  factory BankAccountModel.fromJson(Map<String, dynamic> json) {
    return BankAccountModel(
      id: json['id'] as String? ?? '',
      bankName: json['bank_name'] as String? ?? json['bankName'] as String? ?? '',
      accountNumber: json['account_number'] as String? ?? json['accountNumber'] as String? ?? '',
      accountHolder: json['account_holder'] as String? ?? json['accountHolder'] as String? ?? '',
      isActive: (json['is_active'] as bool?) ?? (json['isActive'] as bool?) ?? true,
    );
  }

  String get maskedNumber {
    if (accountNumber.length <= 4) return accountNumber;
    return '${accountNumber.substring(0, accountNumber.length - 4).replaceAll(RegExp(r'.'), '*')}${accountNumber.substring(accountNumber.length - 4)}';
  }
}

/// Public settings from /api/settings/public endpoint.
class PublicSettingsModel {
  const PublicSettingsModel({
    // App Info
    this.appName = 'ISP Management',
    this.appDescription,
    this.defaultLocale,
    this.appTimezone,
    this.currencyCode,
    this.baseCurrencyCode,
    this.maintenanceMode = false,
    this.maintenanceMessage,
    // Payment Gateways
    this.paymentMidtransEnabled = false,
    this.paymentMidtransClientKey,
    this.paymentMidtransIsProduction = false,
    this.paymentDuitkuEnabled = false,
    this.paymentDuitkuIsProduction = false,
    this.paymentDuitkuPaymentMethods,
    this.paymentManualEnabled = false,
    // Bank accounts
    this.bankAccounts = const [],
  });

  final String appName;
  final String? appDescription;
  final String? defaultLocale;
  final String? appTimezone;
  final String? currencyCode;
  final String? baseCurrencyCode;
  final bool maintenanceMode;
  final String? maintenanceMessage;
  final bool paymentMidtransEnabled;
  final String? paymentMidtransClientKey;
  final bool paymentMidtransIsProduction;
  final bool paymentDuitkuEnabled;
  final bool paymentDuitkuIsProduction;
  final String? paymentDuitkuPaymentMethods;
  final bool paymentManualEnabled;
  final List<BankAccountModel> bankAccounts;

  factory PublicSettingsModel.fromJson(Map<String, dynamic> json) {
    List<BankAccountModel> accounts = [];
    final rawAccounts = json['bank_accounts'] ?? json['bankAccounts'];
    if (rawAccounts is List) {
      accounts = rawAccounts
          .map((a) => BankAccountModel.fromJson(a as Map<String, dynamic>))
          .toList();
    }

    return PublicSettingsModel(
      appName: (json['app_name'] as String?) ?? 'ISP Management',
      appDescription: json['app_description'] as String?,
      defaultLocale: json['default_locale'] as String?,
      appTimezone: json['app_timezone'] as String?,
      currencyCode: json['currency_code'] as String?,
      baseCurrencyCode: json['base_currency_code'] as String?,
      maintenanceMode: json['maintenance_mode'] as bool? ?? false,
      maintenanceMessage: json['maintenance_message'] as String?,
      paymentMidtransEnabled: json['payment_midtrans_enabled'] as bool? ?? false,
      paymentMidtransClientKey: json['payment_midtrans_client_key'] as String?,
      paymentMidtransIsProduction: json['payment_midtrans_is_production'] as bool? ?? false,
      paymentDuitkuEnabled: json['payment_duitku_enabled'] as bool? ?? false,
      paymentDuitkuIsProduction: json['payment_duitku_is_production'] as bool? ?? false,
      paymentDuitkuPaymentMethods: json['payment_duitku_payment_methods'] as String?,
      paymentManualEnabled: json['payment_manual_enabled'] as bool? ?? false,
      bankAccounts: accounts,
    );
  }

  bool get hasActiveBankAccounts =>
      bankAccounts.any((a) => a.isActive);

  List<BankAccountModel> get activeBankAccounts =>
      bankAccounts.where((a) => a.isActive).toList();
}
