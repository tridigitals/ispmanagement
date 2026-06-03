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
  });

  factory TicketModel.fromJson(Map<String, dynamic> json) =>
      _$TicketModelFromJson(json);
  Map<String, dynamic> toJson() => _$TicketModelToJson(this);

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
      ];
}

/// Single chat message in a ticket thread.
@JsonSerializable()
class TicketMessageModel extends Equatable {
  const TicketMessageModel({
    required this.id,
    required this.ticketId,
    required this.body,
    required this.authorName,
    required this.authorRole,
    required this.createdAt,
    this.isFromStaff = false,
  });

  factory TicketMessageModel.fromJson(Map<String, dynamic> json) =>
      _$TicketMessageModelFromJson(json);
  Map<String, dynamic> toJson() => _$TicketMessageToJson(this);

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

  @override
  List<Object?> get props => [id, ticketId, body, authorName, authorRole, createdAt];
}
