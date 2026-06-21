// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ticket_model.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TicketModel _$TicketModelFromJson(Map<String, dynamic> json) => TicketModel(
      id: json['id'] as String,
      subject: json['subject'] as String,
      status: $enumDecode(_$TicketStatusEnumMap, json['status']),
      priority: $enumDecode(_$TicketPriorityEnumMap, json['priority']),
      createdAt: DateTime.parse(json['created_at'] as String),
      updatedAt: DateTime.parse(json['updated_at'] as String),
      description: json['description'] as String?,
      subscriptionId: json['subscription_id'] as String?,
      assignedToName: json['assigned_to_name'] as String?,
      unreadCount: (json['unread_count'] as num?)?.toInt() ?? 0,
      category: json['category'] as String?,
      satisfactionRating: (json['satisfaction_rating'] as num?)?.toInt(),
      satisfactionComment: json['satisfaction_comment'] as String?,
      startedAt: json['started_at'] == null
          ? null
          : DateTime.parse(json['started_at'] as String),
      resolvedAt: json['resolved_at'] == null
          ? null
          : DateTime.parse(json['resolved_at'] as String),
      completionNotes: json['completion_notes'] as String?,
      signatureUrl: json['signature_url'] as String?,
      completionPhotos: (json['completion_photos'] as List<dynamic>?)
              ?.map((e) => e as String)
              .toList() ??
          const [],
    );

Map<String, dynamic> _$TicketModelToJson(TicketModel instance) =>
    <String, dynamic>{
      'id': instance.id,
      'subject': instance.subject,
      'status': _$TicketStatusEnumMap[instance.status]!,
      'priority': _$TicketPriorityEnumMap[instance.priority]!,
      'created_at': instance.createdAt.toIso8601String(),
      'updated_at': instance.updatedAt.toIso8601String(),
      'description': instance.description,
      'subscription_id': instance.subscriptionId,
      'assigned_to_name': instance.assignedToName,
      'unread_count': instance.unreadCount,
      'category': instance.category,
      'satisfaction_rating': instance.satisfactionRating,
      'satisfaction_comment': instance.satisfactionComment,
      'started_at': instance.startedAt?.toIso8601String(),
      'resolved_at': instance.resolvedAt?.toIso8601String(),
      'completion_notes': instance.completionNotes,
      'signature_url': instance.signatureUrl,
      'completion_photos': instance.completionPhotos,
    };

const _$TicketStatusEnumMap = {
  TicketStatus.open: 'open',
  TicketStatus.inProgress: 'in_progress',
  TicketStatus.waitingCustomer: 'waiting_customer',
  TicketStatus.waitingStaff: 'waiting_staff',
  TicketStatus.resolved: 'resolved',
  TicketStatus.closed: 'closed',
  TicketStatus.cancelled: 'cancelled',
};

const _$TicketPriorityEnumMap = {
  TicketPriority.low: 'low',
  TicketPriority.normal: 'normal',
  TicketPriority.high: 'high',
  TicketPriority.urgent: 'urgent',
};

TicketAttachmentModel _$TicketAttachmentModelFromJson(
        Map<String, dynamic> json) =>
    TicketAttachmentModel(
      id: json['id'] as String,
      name: json['name'] as String,
      originalName: json['original_name'] as String,
      size: (json['size'] as num).toInt(),
      contentType: json['content_type'] as String,
    );

Map<String, dynamic> _$TicketAttachmentModelToJson(
        TicketAttachmentModel instance) =>
    <String, dynamic>{
      'id': instance.id,
      'name': instance.name,
      'original_name': instance.originalName,
      'size': instance.size,
      'content_type': instance.contentType,
    };

TicketMessageModel _$TicketMessageModelFromJson(Map<String, dynamic> json) =>
    TicketMessageModel(
      id: json['id'] as String,
      ticketId: json['ticket_id'] as String,
      body: json['body'] as String,
      createdAt: DateTime.parse(json['created_at'] as String),
      authorName: json['author_name'] as String? ?? 'anonymous',
      authorRole: json['author_role'] as String? ?? 'customer',
      isFromStaff: json['is_from_staff'] as bool? ?? false,
      attachments: (json['attachments'] as List<dynamic>?)
              ?.map((e) =>
                  TicketAttachmentModel.fromJson(e as Map<String, dynamic>))
              .toList() ??
          const [],
    );

Map<String, dynamic> _$TicketMessageModelToJson(TicketMessageModel instance) =>
    <String, dynamic>{
      'id': instance.id,
      'ticket_id': instance.ticketId,
      'body': instance.body,
      'author_name': instance.authorName,
      'author_role': instance.authorRole,
      'created_at': instance.createdAt.toIso8601String(),
      'is_from_staff': instance.isFromStaff,
      'attachments': instance.attachments,
    };
