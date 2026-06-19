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
        return 'open';
      case TicketStatus.inProgress:
        return 'inProgress';
      case TicketStatus.waitingCustomer:
        return 'waitingCustomer';
      case TicketStatus.waitingStaff:
        return 'waitingStaff';
      case TicketStatus.resolved:
        return 'resolved';
      case TicketStatus.closed:
        return 'closed';
      case TicketStatus.cancelled:
        return 'cancelled';
    }
  }

  String priorityLabel() {
    switch (priority) {
      case TicketPriority.low:
        return 'low';
      case TicketPriority.normal:
        return 'normal';
      case TicketPriority.high:
        return 'high';
      case TicketPriority.urgent:
        return 'urgent';
    }
  }

  String categoryLabel() {
    switch (category) {
      case 'general':
        return 'general';
      case 'billing':
        return 'billing';
      case 'technical':
        return 'technical';
      case 'installation':
        return 'installation';
      default:
        return 'general';
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

  /// Resolved content type — falls back to extension inference when the
  /// server-stored value is empty or `application/octet-stream` (common for
  /// mobile pickers that don't set a precise multipart Content-Type).
  String get resolvedContentType {
    final ct = contentType.trim();
    if (ct.isNotEmpty && ct != 'application/octet-stream') return ct;
    return _inferContentTypeFromExt(originalName.isNotEmpty ? originalName : name);
  }

  /// Whether this is an image file (uses [resolvedContentType]).
  bool get isImage => resolvedContentType.startsWith('image/');

  /// Whether this is a video file (uses [resolvedContentType]).
  bool get isVideo => resolvedContentType.startsWith('video/');

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
    this.authorName = 'anonymous',
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
    // Prefer the API-supplied author_name (server resolves it from
    // users.name at message-create time). Fall back to a generic i18n
    // key only when the API didn't include one (e.g. messages created
    // before the author_name column was added). The UI looks up the key
    // via the l10n extension (see ticket_l10n.dart).
    final apiName = sanitized['author_name'] as String?;
    if (apiName == null || apiName.isEmpty || apiName == 'Pelanggan') {
      sanitized['author_name'] = isStaff ? 'staff' : 'customer';
    }
    sanitized['author_role'] = isStaff ? 'staff' : 'customer';
    return _$TicketMessageModelFromJson(sanitized);
  }

  Map<String, dynamic> toJson() => _$TicketMessageModelToJson(this);

  static Map<String, dynamic> _sanitizeMessageJson(Map<String, dynamic> json) {
    if (!json.containsKey('author_name') || json['author_name'] == null) {
      json['author_name'] = 'customer';
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

/// Map common file extensions to MIME types — mirrors the backend helper.
/// Used as a fallback when the server stores `application/octet-stream`.
String _inferContentTypeFromExt(String filename) {
  final ext = filename.contains('.')
      ? filename.split('.').last.toLowerCase()
      : '';
  switch (ext) {
    case 'jpg':
    case 'jpeg':
      return 'image/jpeg';
    case 'png':
      return 'image/png';
    case 'gif':
      return 'image/gif';
    case 'webp':
      return 'image/webp';
    case 'bmp':
      return 'image/bmp';
    case 'svg':
      return 'image/svg+xml';
    case 'heic':
    case 'heif':
      return 'image/heic';
    case 'mp4':
      return 'video/mp4';
    case 'mov':
      return 'video/quicktime';
    case 'webm':
      return 'video/webm';
    case 'mkv':
      return 'video/x-matroska';
    case 'avi':
      return 'video/x-msvideo';
    case '3gp':
      return 'video/3gpp';
    case 'mp3':
      return 'audio/mpeg';
    case 'm4a':
      return 'audio/mp4';
    case 'wav':
      return 'audio/wav';
    case 'ogg':
      return 'audio/ogg';
    case 'pdf':
      return 'application/pdf';
    default:
      return 'application/octet-stream';
  }
}
