import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'invoice_model.g.dart';

/// Invoice status values mirror backend.
enum InvoiceStatus {
  @JsonValue('unpaid')
  unpaid,
  @JsonValue('paid')
  paid,
  @JsonValue('overdue')
  overdue,
  @JsonValue('partial')
  partial,
  @JsonValue('cancelled')
  cancelled,
  @JsonValue('refunded')
  refunded,
  @JsonValue('pending')
  pending,
}

/// Customer invoice (tagihan).
@JsonSerializable()
class InvoiceModel extends Equatable {
  const InvoiceModel({
    required this.id,
    required this.invoiceNumber,
    required this.amount,
    required this.amountPaid,
    required this.currencyCode,
    required this.status,
    required this.dueDate,
    required this.createdAt,
    this.subscriptionId,
    this.subscriptionLabel,
    this.paidAt,
    this.notes,
  });

  factory InvoiceModel.fromJson(Map<String, dynamic> json) =>
      _$InvoiceModelFromJson(json);
  Map<String, dynamic> toJson() => _$InvoiceModelToJson(this);

  final String id;
  @JsonKey(name: 'invoice_number')
  final String invoiceNumber;
  final double amount;
  @JsonKey(name: 'amount_paid')
  final double amountPaid;
  @JsonKey(name: 'currency_code')
  final String currencyCode;
  final InvoiceStatus status;
  @JsonKey(name: 'due_date')
  final DateTime dueDate;
  @JsonKey(name: 'created_at')
  final DateTime createdAt;
  @JsonKey(name: 'subscription_id')
  final String? subscriptionId;
  @JsonKey(name: 'subscription_label')
  final String? subscriptionLabel;
  @JsonKey(name: 'paid_at')
  final DateTime? paidAt;
  final String? notes;

  double get amountRemaining => (amount - amountPaid).clamp(0, double.infinity);

  bool get isPaid => status == InvoiceStatus.paid;
  bool get isUnpaid => status == InvoiceStatus.unpaid;
  bool get isOverdue =>
      status == InvoiceStatus.overdue ||
      (isUnpaid && DateTime.now().isAfter(dueDate));

  String statusLabel() {
    switch (status) {
      case InvoiceStatus.unpaid:
        return 'Belum Bayar';
      case InvoiceStatus.paid:
        return 'Lunas';
      case InvoiceStatus.overdue:
        return 'Jatuh Tempo';
      case InvoiceStatus.partial:
        return 'Sebagian';
      case InvoiceStatus.cancelled:
        return 'Dibatalkan';
      case InvoiceStatus.refunded:
        return 'Dikembalikan';
      case InvoiceStatus.pending:
        return 'Menunggu';
    }
  }

  @override
  List<Object?> get props => [
        id,
        invoiceNumber,
        amount,
        amountPaid,
        currencyCode,
        status,
        dueDate,
        createdAt,
        subscriptionId,
        paidAt,
      ];
}

/// Payment record.
@JsonSerializable()
class PaymentModel extends Equatable {
  const PaymentModel({
    required this.id,
    required this.invoiceId,
    required this.amount,
    required this.method,
    required this.status,
    required this.createdAt,
    this.reference,
    this.notes,
  });

  factory PaymentModel.fromJson(Map<String, dynamic> json) =>
      _$PaymentModelFromJson(json);
  Map<String, dynamic> toJson() => _$PaymentModelToJson(this);

  final String id;
  @JsonKey(name: 'invoice_id')
  final String invoiceId;
  final double amount;
  final String method;
  final String status;
  @JsonKey(name: 'created_at')
  final DateTime createdAt;
  final String? reference;
  final String? notes;

  @override
  List<Object?> get props => [id, invoiceId, amount, method, status, createdAt];
}
