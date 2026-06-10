import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

part 'ticket_model.g.dart';

enum TicketStatus {
  @JsonValue('open')
  open,
  @JsonValue('in_progress')
  inProgress,
  @JsonValue('waiting_customer')
  waitingCustomer,
  @JsonValue('waiting_staff')
  waitingStaff,
  @JsonValue('resolved')
  resolved,
  @JsonValue('closed')
  closed,
  @JsonValue('cancelled')
  cancelled,
}

enum TicketPriority {
  @JsonValue('low')
  low,
  @JsonValue('normal')
  normal,
  @JsonValue('high')
  high,
  @JsonValue('urgent')
  urgent,
}

enum TicketCategory {
  @JsonValue('general')
  general,
  @JsonValue('billing')
  billing,
  @JsonValue('technical')
  technical,
  @JsonValue('installation')
  installation,
}

/// Support ticket (tiket gangguan / permintaan).
@JsonSerializable()
class TicketModel extends Equatable {
  const TicketModel({
    required this.id,
    required this.subject,
    required this.status,
    required this.priority,
    required this.createdAt,
    required this.updatedAt,
    this.description,
    this.subscriptionId,
    this.assignedToName,
    this.unreadCount = 0,
    this.category,
    this.satisfactionRating,
    this.satisfactionComment,
  });

  factory TicketModel.fromJson(Map<String, dynamic> json) =>
      _$TicketModelFromJson(_sanitizeTicketJson(json));
  Map<String, dynamic> toJson() => _$TicketModelToJson(this);

  static Map<String, dynamic> _sanitizeTicketJson(Map<String, dynamic> json) {
    if (!json.containsKey('assigned_to_name') && json.containsKey('assigned_to')) {
      json['assigned_to_name'] = json['assigned_to']?.toString();
    }
    return json;
  }

  final String id;
  final String subject;
  final TicketStatus status;
  final TicketPriority priority;
  @JsonKey(name: 'created_at')
  final DateTime createdAt;
  @JsonKey(name: 'updated_at')
  final DateTime updatedAt;
  final String? description;
  @JsonKey(name: 'subscription_id')
  final String? subscriptionId;
  @JsonKey(name: 'assigned_to_name')
  final String? assignedToName;
  @JsonKey(name: 'unread_count')
  final int unreadCount;
  final String? category;
  @JsonKey(name: 'satisfaction_rating')
  final int? satisfactionRating;
  @JsonKey(name: 'satisfaction_comment')
  final String? satisfactionComment;

  bool get isOpen => status == TicketStatus.open || status == TicketStatus.inProgress;
  bool get isClosed =>
      status == TicketStatus.closed ||
      status == TicketStatus.cancelled ||
      status == TicketStatus.resolved;

  String statusLabel() {
    switch (status) {
      case TicketStatus.open:
        return 'Terbuka';
      case TicketStatus.inProgress:
        return 'Ditangani';
      case TicketStatus.waitingCustomer:
        return 'Menunggu Pelanggan';
      case TicketStatus.waitingStaff:
        return 'Menunggu Tim';
      case TicketStatus.resolved:
        return 'Selesai';
      case TicketStatus.closed:
        return 'Ditutup';
      case TicketStatus.cancelled:
        return 'Dibatalkan';
    }
  }

  String priorityLabel() {
    switch (priority) {
      case TicketPriority.low:
        return 'Rendah';
      case TicketPriority.normal:
        return 'Normal';
      case TicketPriority.high:
        return 'Tinggi';
      case TicketPriority.urgent:
        return 'Mendesak';
    }
  }

  String categoryLabel() {
    switch (category) {
      case 'general':
        return 'Umum';
      case 'billing':
        return 'Tagihan';
      case 'technical':
        return 'Teknis';
      case 'installation':
        return 'Instalasi';
      default:
        return 'Umum';
    }
  }

  @override
  List<Object?> get props => [
        id,
        subject,
        status,
        priority,
        createdAt,
        updatedAt,
        subscriptionId,
        assignedToName,
        unreadCount,
        category,
        satisfactionRating,
        satisfactionComment,
      ];
}

/// File attachment in a ticket message.
@JsonSerializable()
class TicketAttachmentModel extends Equatable {
  const TicketAttachmentModel({
    required this.id,
    required this.name,
    required this.originalName,
    required this.size,
    required this.contentType,
  });

  factory TicketAttachmentModel.fromJson(Map<String, dynamic> json) {
    return _$TicketAttachmentModelFromJson(json);
  }

  final String id;
  final String name;
  @JsonKey(name: 'original_name')
  final String originalName;
  final int size;
  @JsonKey(name: 'content_type')
  final String contentType;

  /// Whether this is an image file.
  bool get isImage => contentType.startsWith('image/');

  /// Human-readable file size.
  String get humanSize {
    if (size < 1024) return '$size B';
    if (size < 1024 * 1024) return '${(size / 1024).toStringAsFixed(1)} KB';
    return '${(size / (1024 * 1024)).toStringAsFixed(1)} MB';
  }

  Map<String, dynamic> toJson() => _$TicketAttachmentModelToJson(this);

  @override
  List<Object?> get props => [id, name, originalName, size, contentType];
}

/// Single chat message in a ticket thread.
@JsonSerializable()
class TicketMessageModel extends Equatable {
  const TicketMessageModel({
    required this.id,
    required this.ticketId,
    required this.body,
    required this.createdAt,
    this.authorName = 'Anonim',
    this.authorRole = 'customer',
    this.authorId,
    this.isFromStaff = false,
    this.attachments = const [],
  });

  factory TicketMessageModel.fromJson(Map<String, dynamic> json) =>
      _$TicketMessageModelFromJson(_sanitizeMessageJson(json));

  factory TicketMessageModel.fromTicketJson(
    Map<String, dynamic> json,
    String currentUserId,
  ) {
    final sanitized = _sanitizeMessageJson(json);
    final authorId = sanitized['author_id'] as String?;
    final isStaff = authorId != null && authorId != currentUserId;
    sanitized['is_from_staff'] = isStaff;
    if (isStaff) {
      sanitized['author_name'] = 'Admin';
      sanitized['author_role'] = 'staff';
    } else {
      sanitized['author_name'] = 'Anda';
      sanitized['author_role'] = 'customer';
    }
    return _$TicketMessageModelFromJson(sanitized);
  }

  Map<String, dynamic> toJson() => _$TicketMessageModelToJson(this);

  static Map<String, dynamic> _sanitizeMessageJson(Map<String, dynamic> json) {
    if (!json.containsKey('author_name') || json['author_name'] == null) {
      json['author_name'] = 'Pelanggan';
    }
    if (!json.containsKey('author_role') || json['author_role'] == null) {
      json['author_role'] = 'customer';
    }
    if (!json.containsKey('is_from_staff') || json['is_from_staff'] == null) {
      json['is_from_staff'] = false;
    }
    // Parse attachments list
    if (!json.containsKey('attachments')) {
      json['attachments'] = [];
    }
    return json;
  }

  final String id;
  @JsonKey(name: 'ticket_id')
  final String ticketId;
  final String body;
  @JsonKey(name: 'author_name')
  final String authorName;
  @JsonKey(name: 'author_role')
  final String authorRole;
  @JsonKey(name: 'created_at')
  final DateTime createdAt;
  @JsonKey(name: 'is_from_staff')
  final bool isFromStaff;
  final List<TicketAttachmentModel> attachments;

  @JsonKey(includeFromJson: false, includeToJson: false)
  final String? authorId;

  @override
  List<Object?> get props => [id, ticketId, body, authorName, authorRole, createdAt];
}
