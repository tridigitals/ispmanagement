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
    this.actionUrl,
    this.imageUrl,
    this.data,
    this.readAt,
  });

  factory NotificationModel.fromJson(Map<String, dynamic> json) =>
      _$NotificationModelFromJson(_sanitizeJson(json));

  static Map<String, dynamic> _sanitizeJson(Map<String, dynamic> json) {
    final rawCategory = (json['category'] ?? 'system').toString().trim().toLowerCase();
    final normalizedCategory = switch (rawCategory) {
      'billing' => 'invoice',
      'bill' => 'invoice',
      'invoice' => 'invoice',
      'ticket' => 'ticket',
      'support' => 'ticket',
      'outage' => 'outage',
      'payment' => 'payment',
      'subscription' => 'subscription',
      'promo' => 'promo',
      'system' => 'system',
      _ => 'system',
    };

    return <String, dynamic>{
      'id': (json['id'] ?? json['notification_id'] ?? '').toString(),
      'title': (json['title'] ?? json['subject'] ?? 'Notifikasi').toString(),
      'body': (json['body'] ?? json['message'] ?? '').toString(),
      'category': normalizedCategory,
      'createdAt': json['created_at'] ?? json['createdAt'] ?? DateTime.now().toIso8601String(),
      'deepLink': json['deep_link'] ?? json['deepLink'],
      'actionUrl': json['action_url'] ?? json['actionUrl'],
      'imageUrl': json['image_url'] ?? json['imageUrl'],
      'data': json['data'] is Map<String, dynamic> ? json['data'] : null,
      'readAt': json['read_at'] ?? json['readAt'] ?? (json['is_read'] == true ? DateTime.now().toIso8601String() : null),
    };
  }

  final String id;
  final String title;
  final String body;
  final NotificationCategory category;
  final DateTime createdAt;
  final String? deepLink;
  final String? actionUrl;
  final String? imageUrl;
  final Map<String, dynamic>? data;
  final DateTime? readAt;

  String get categoryLabel => category.label;
  bool get isUnread => readAt == null;

  Map<String, dynamic> toJson() => _$NotificationModelToJson(this);

  NotificationModel copyWith({
    String? id,
    String? title,
    String? body,
    NotificationCategory? category,
    DateTime? createdAt,
    String? deepLink,
    String? actionUrl,
    String? imageUrl,
    Map<String, dynamic>? data,
    DateTime? readAt,
  }) {
    return NotificationModel(
      id: id ?? this.id,
      title: title ?? this.title,
      body: body ?? this.body,
      category: category ?? this.category,
      createdAt: createdAt ?? this.createdAt,
      deepLink: deepLink ?? this.deepLink,
      actionUrl: actionUrl ?? this.actionUrl,
      imageUrl: imageUrl ?? this.imageUrl,
      data: data ?? this.data,
      readAt: readAt ?? this.readAt,
    );
  }

  @override
  List<Object?> get props => [id, title, body, category, createdAt, readAt];
}
