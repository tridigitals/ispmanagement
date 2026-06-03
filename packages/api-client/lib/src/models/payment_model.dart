import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'payment_model.g.dart';

/// Payment transaction status from the payment gateway (e.g. Midtrans).
enum PaymentStatus {
  @JsonValue('pending')
  pending,
  @JsonValue('paid')
  paid,
  @JsonValue('failed')
  failed,
  @JsonValue('expired')
  expired,
  @JsonValue('cancelled')
  cancelled,
  @JsonValue('refunded')
  refunded,
}

extension on PaymentStatus {
  String get label {
    switch (this) {
      case PaymentStatus.pending:
        return 'Menunggu';
      case PaymentStatus.paid:
        return 'Lunas';
      case PaymentStatus.failed:
        return 'Gagal';
      case PaymentStatus.expired:
        return 'Kedaluwarsa';
      case PaymentStatus.cancelled:
        return 'Dibatalkan';
      case PaymentStatus.refunded:
        return 'Dikembalikan';
    }
  }
}

enum PaymentMethod {
  @JsonValue('virtual_account')
  virtualAccount,
  @JsonValue('e_wallet')
  ewallet,
  @JsonValue('qris')
  qris,
  @JsonValue('credit_card')
  creditCard,
  @JsonValue('bank_transfer')
  bankTransfer,
  @JsonValue('cstore')
  convenienceStore,
  @JsonValue('unknown')
  unknown,
}

extension on PaymentMethod {
  String get label {
    switch (this) {
      case PaymentMethod.virtualAccount:
        return 'Virtual Account';
      case PaymentMethod.ewallet:
        return 'E-Wallet';
      case PaymentMethod.qris:
        return 'QRIS';
      case PaymentMethod.creditCard:
        return 'Kartu Kredit';
      case PaymentMethod.bankTransfer:
        return 'Transfer Bank';
      case PaymentMethod.convenienceStore:
        return 'Indomaret/Alfamart';
      case PaymentMethod.unknown:
        return 'Lainnya';
    }
  }
}

@JsonSerializable()
class PaymentChannel extends Equatable {
  const PaymentChannel({
    required this.code,
    required this.name,
    required this.method,
    this.fee = 0,
    this.iconUrl,
    this.logoUrl,
  });

  factory PaymentChannel.fromJson(Map<String, dynamic> json) =>
      _$PaymentChannelFromJson(json);

  final String code;
  final String name;
  final PaymentMethod method;
  final double fee;
  final String? iconUrl;
  final String? logoUrl;

  String get methodLabel => method.label;

  Map<String, dynamic> toJson() => _$PaymentChannelToJson(this);

  @override
  List<Object?> get props => [code, name, method, fee];
}

@JsonSerializable()
class PaymentTransaction extends Equatable {
  const PaymentTransaction({
    required this.id,
    required this.invoiceId,
    required this.amount,
    required this.status,
    required this.method,
    required this.createdAt,
    this.paidAt,
    this.expiredAt,
    this.gatewayName = 'midtrans',
    this.gatewayRef,
    this.paymentCode,
    this.paymentUrl,
    this.qrCodeUrl,
    this.vaNumber,
    this.actions,
  });

  factory PaymentTransaction.fromJson(Map<String, dynamic> json) =>
      _$PaymentTransactionFromJson(json);

  final String id;
  final String invoiceId;
  final double amount;
  final PaymentStatus status;
  final PaymentMethod method;
  final DateTime createdAt;
  final DateTime? paidAt;
  final DateTime? expiredAt;
  final String gatewayName;
  final String? gatewayRef;
  final String? paymentCode;
  final String? paymentUrl;
  final String? qrCodeUrl;
  final String? vaNumber;
  final List<PaymentAction>? actions;

  bool get isPending => status == PaymentStatus.pending;
  bool get isPaid => status == PaymentStatus.paid;
  bool get isFailed => status == PaymentStatus.failed;
  bool get isExpired => status == PaymentStatus.expired;

  /// Whether we still have time to pay.
  bool get isPayable {
    if (status != PaymentStatus.pending) return false;
    if (expiredAt == null) return true;
    return DateTime.now().isBefore(expiredAt!);
  }

  String get statusLabel => status.label;

  Map<String, dynamic> toJson() => _$PaymentTransactionToJson(this);

  @override
  List<Object?> get props => [
        id,
        invoiceId,
        amount,
        status,
        method,
        createdAt,
        paidAt,
        expiredAt,
      ];
}

@JsonSerializable()
class PaymentAction extends Equatable {
  const PaymentAction({
    required this.name,
    required this.method,
    required this.url,
  });

  factory PaymentAction.fromJson(Map<String, dynamic> json) =>
      _$PaymentActionFromJson(json);

  final String name;
  final String method; // 'GET', 'POST', etc.
  final String url;

  Map<String, dynamic> toJson() => _$PaymentActionToJson(this);

  @override
  List<Object?> get props => [name, method, url];
}
