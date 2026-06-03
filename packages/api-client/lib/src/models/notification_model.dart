import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'notification_model.g.dart';

enum NotificationCategory {
  @JsonValue('invoice')
  invoice,
  @JsonValue('ticket')
  ticket,
  @JsonValue('outage')
  outage,
  @JsonValue('payment')
  payment,
  @JsonValue('subscription')
  subscription,
  @JsonValue('promo')
  promo,
  @JsonValue('system')
  system,
}

extension on NotificationCategory {
  String get label {
    switch (this) {
      case NotificationCategory.invoice:
        return 'Tagihan';
      case NotificationCategory.ticket:
        return 'Tiket';
      case NotificationCategory.outage:
        return 'Gangguan';
      case NotificationCategory.payment:
        return 'Pembayaran';
      case NotificationCategory.subscription:
        return 'Langganan';
      case NotificationCategory.promo:
        return 'Promo';
      case NotificationCategory.system:
        return 'Sistem';
    }
  }
}

@JsonSerializable()
class NotificationModel extends Equatable {
  const NotificationModel({
    required this.id,
    required this.title,
    required this.body,
    required this.category,
    required this.createdAt,
    this.deepLink,
    this.imageUrl,
    this.data,
    this.readAt,
  });

  factory NotificationModel.fromJson(Map<String, dynamic> json) =>
      _$NotificationModelFromJson(json);

  final String id;
  final String title;
  final String body;
  final NotificationCategory category;
  final DateTime createdAt;
  final String? deepLink;
  final String? imageUrl;
  final Map<String, dynamic>? data;
  final DateTime? readAt;

  String get categoryLabel => category.label;
  bool get isUnread => readAt == null;

  Map<String, dynamic> toJson() => _$NotificationModelToJson(this);

  @override
  List<Object?> get props => [id, title, body, category, createdAt, readAt];
}
