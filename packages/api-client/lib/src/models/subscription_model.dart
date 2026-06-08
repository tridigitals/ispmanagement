import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'subscription_model.g.dart';

/// Status of a customer subscription.
enum SubscriptionStatus {
  @JsonValue('active')
  active,
  @JsonValue('pending_installation')
  pendingInstallation,
  @JsonValue('suspended')
  suspended,
  @JsonValue('cancelled')
  cancelled,
  @JsonValue('grace')
  grace,
  @JsonValue('expired')
  expired,
  @JsonValue('unknown')
  unknown,
}

/// Customer subscription — internet service the customer is paying for.
@JsonSerializable()
class SubscriptionModel extends Equatable {
  const SubscriptionModel({
    required this.id,
    required this.tenantId,
    required this.customerId,
    required this.status,
    required this.billingCycle,
    required this.price,
    required this.currencyCode,
    this.packageId,
    this.packageName,
    this.locationId,
    this.locationLabel,
    this.routerId,
    this.routerName,
    this.startsAt,
    this.endsAt,
    this.graceUntil,
    this.notes,
  });

  factory SubscriptionModel.fromJson(Map<String, dynamic> json) =>
      _$SubscriptionModelFromJson(json);
  Map<String, dynamic> toJson() => _$SubscriptionModelToJson(this);

  final String id;
  @JsonKey(name: 'tenant_id')
  final String tenantId;
  @JsonKey(name: 'customer_id')
  final String customerId;
  final SubscriptionStatus status;
  @JsonKey(name: 'billing_cycle')
  final String billingCycle;
  final double price;
  @JsonKey(name: 'currency_code')
  final String currencyCode;
  @JsonKey(name: 'package_id')
  final String? packageId;
  @JsonKey(name: 'package_name')
  final String? packageName;
  @JsonKey(name: 'location_id')
  final String? locationId;
  @JsonKey(name: 'location_label')
  final String? locationLabel;
  @JsonKey(name: 'router_id')
  final String? routerId;
  @JsonKey(name: 'router_name')
  final String? routerName;
  @JsonKey(name: 'starts_at')
  final DateTime? startsAt;
  @JsonKey(name: 'ends_at')
  final DateTime? endsAt;
  @JsonKey(name: 'grace_until')
  final DateTime? graceUntil;
  final String? notes;

  bool get isActive => status == SubscriptionStatus.active;
  bool get isSuspended => status == SubscriptionStatus.suspended;
  bool get needsAttention =>
      status == SubscriptionStatus.suspended || status == SubscriptionStatus.cancelled;

  String statusLabel() {
    switch (status) {
      case SubscriptionStatus.active:
        return 'Aktif';
      case SubscriptionStatus.pendingInstallation:
        return 'Menunggu Pemasangan';
      case SubscriptionStatus.suspended:
        return 'Ditangguhkan';
      case SubscriptionStatus.cancelled:
        return 'Dibatalkan';
      case SubscriptionStatus.grace:
        return 'Masa Tenggang';
      case SubscriptionStatus.expired:
        return 'Kedaluwarsa';
      case SubscriptionStatus.unknown:
        return 'Tidak Diketahui';
    }
  }

  @override
  List<Object?> get props => [
        id,
        tenantId,
        customerId,
        status,
        billingCycle,
        price,
        currencyCode,
        packageId,
        packageName,
        locationId,
        locationLabel,
        startsAt,
        endsAt,
        graceUntil,
      ];
}
