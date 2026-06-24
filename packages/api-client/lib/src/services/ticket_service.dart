import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../models/paginated_response.dart';
import '../api/api_endpoints.dart';
import '../models/ticket_model.dart';
import 'auth_service.dart';

/// Response from POST /api/support/tickets/{id}/photos (multipart upload).
/// Only `id` is consumed by the technician app for inclusion in resolve call.
class TicketPhotoUploadResult {
  TicketPhotoUploadResult({required this.id, required this.url});
  final String id;
  final String url;

  factory TicketPhotoUploadResult.fromJson(Map<String, dynamic> json) {
    return TicketPhotoUploadResult(
      id: (json['id'] ?? '') as String,
      url: (json['url'] ?? json['path'] ?? '') as String,
    );
  }
}

class TicketService {
  TicketService({required this.dio});
  final Dio dio;

  Future<ServiceResult<PaginatedResponse<TicketModel>>> list({
    int page = 1,
    int perPage = 20,
    String? status,
    String? category,
  }) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(
        ApiEndpoints.myTickets,
        queryParameters: {
          'page': page,
          'per_page': perPage,
          if (status != null) 'status': status,
          if (category != null) 'category': category,
        },
      );
      return PaginatedResponse<TicketModel>.fromJson(
        res.data ?? const {},
        TicketModel.fromJson,
      );
    });
  }

  Future<ServiceResult<TicketModel>> getById(String id) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(
        ApiEndpoints.myTicketById(id),
      );
      final data = res.data ?? const {};
      final ticketJson = data['ticket'] is Map<String, dynamic>
          ? data['ticket'] as Map<String, dynamic>
          : data;
      return TicketModel.fromJson(ticketJson);
    });
  }

  Future<ServiceResult<TicketModel>> create({
    required String subject,
    required String message,
    String priority = 'normal',
    String? category,
    String? subscriptionId,
    List<String>? attachmentIds,
  }) async {
    return _execute(() async {
      final body = <String, dynamic>{
        'subject': subject,
        'message': message,
        'priority': priority,
      };
      if (category != null) body['category'] = category;
      if (subscriptionId != null) body['subscription_id'] = subscriptionId;
      if (attachmentIds != null && attachmentIds.isNotEmpty) {
        body['attachment_ids'] = attachmentIds;
      }
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.createTicket,
        data: body,
      );
      final responseData = res.data ?? const {};
      final ticketJson = responseData['ticket'] is Map<String, dynamic>
          ? responseData['ticket'] as Map<String, dynamic>
          : responseData;
      return TicketModel.fromJson(ticketJson);
    });
  }

  /// Fetch messages for a ticket from the dedicated messages endpoint.
  /// [currentUserId] is used to determine which messages are from staff vs customer.
  Future<ServiceResult<List<TicketMessageModel>>> listMessages(
    String ticketId, {
    String? currentUserId,
  }) async {
    return _execute(() async {
      final res = await dio.get<List<dynamic>>(
        ApiEndpoints.ticketMessages(ticketId),
      );
      final list = res.data ?? const [];
      return list
          .whereType<Map<String, dynamic>>()
          .map((json) => currentUserId != null
              ? TicketMessageModel.fromTicketJson(json, currentUserId)
              : TicketMessageModel.fromJson(json))
          .toList();
    });
  }

  Future<ServiceResult<TicketMessageModel>> reply({
    required String ticketId,
    required String message,
    List<String>? attachmentIds,
  }) async {
    return _execute(() async {
      final body = <String, dynamic>{'message': message};
      if (attachmentIds != null && attachmentIds.isNotEmpty) {
        body['attachment_ids'] = attachmentIds;
      }
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.ticketMessages(ticketId),
        data: body,
      );
      return TicketMessageModel.fromJson(res.data ?? const {});
    });
  }

  /// Fetch ticket stats. Backend filters by role:
  /// - admin/staff (with `support:read_all`): all tickets in tenant
  /// - technician (with `support:read`): tickets assigned to them
  /// - customer: tickets created by them
  Future<ServiceResult<TicketStats>> stats() async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(ApiEndpoints.ticketStats);
      return TicketStats.fromJson(res.data ?? const {});
    });
  }

  Future<ServiceResult<void>> submitSatisfaction({
    required String ticketId,
    required int rating,
    String? comment,
  }) async {
    return _execute(() async {
      await dio.post<dynamic>(
        ApiEndpoints.ticketSatisfaction(ticketId),
        data: {
          'rating': rating,
          if (comment != null && comment.isNotEmpty) 'comment': comment,
        },
      );
    });
  }

  /// Update a ticket's mutable fields (admin action). Backend accepts
  /// any subset of: status, priority, category, assigned_to.
  Future<ServiceResult<TicketModel>> update(
    String ticketId, {
    String? status,
    String? priority,
    String? category,
    String? assignedTo,
  }) async {
    return _execute(() async {
      final body = <String, dynamic>{
        if (status != null) 'status': status,
        if (priority != null) 'priority': priority,
        if (category != null) 'category': category,
        if (assignedTo != null) 'assigned_to': assignedTo,
      };
      if (body.isEmpty) {
        throw ArgumentError('update() requires at least one field to change');
      }
      final res = await dio.put<Map<String, dynamic>>(
        ApiEndpoints.myTicketById(ticketId),
        data: body,
      );
      return TicketModel.fromJson(res.data ?? const {});
    });
  }

  /// Upload a proof-of-work photo (multipart). Returns the file_record id.
  /// The technician app captures the photo with image_picker, then submits
  /// the local file path here. The id is included in resolve()'s
  /// `photoFileIds` parameter to attach the photo to the resolve event.
  Future<ServiceResult<TicketPhotoUploadResult>> uploadPhoto({
    required String ticketId,
    required String filePath,
    String? filename,
  }) async {
    return _execute(() async {
      final form = FormData.fromMap({
        'photo': await MultipartFile.fromFile(
          filePath,
          filename: filename ?? filePath.split('/').last,
        ),
      });
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.ticketPhotoUpload(ticketId),
        data: form,
        options: Options(contentType: 'multipart/form-data'),
      );
      return TicketPhotoUploadResult.fromJson(res.data ?? const {});
    });
  }

  /// Mark a ticket as in_progress. Only the assigned technician/admin can call.
  Future<ServiceResult<TicketModel>> startTicket(String ticketId) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.ticketStart(ticketId),
      );
      return TicketModel.fromJson(res.data ?? const {});
    });
  }

  /// Resolve a ticket with completion proof.
  ///
  /// [completionNotes] free-text notes from the technician.
  /// [photoFileIds] file_record ids returned by uploadPhoto().
  /// [signatureFileId] file_record id of a PNG signature image (optional).
  Future<ServiceResult<TicketModel>> resolveTicket({
    required String ticketId,
    String? completionNotes,
    List<String>? photoFileIds,
    String? signatureFileId,
  }) async {
    return _execute(() async {
      final body = <String, dynamic>{
        if (completionNotes != null && completionNotes.isNotEmpty)
          'completion_notes': completionNotes,
        if (photoFileIds != null && photoFileIds.isNotEmpty)
          'photo_file_ids': photoFileIds,
        if (signatureFileId != null) 'signature_file_id': signatureFileId,
      };
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.ticketResolve(ticketId),
        data: body,
      );
      return TicketModel.fromJson(res.data ?? const {});
    });
  }

  /// Claim an unassigned ticket — race-safe (UPDATE WHERE assigned_to IS NULL).
  /// Returns the updated ticket if successful, or an error if already assigned.
  Future<ServiceResult<TicketModel>> claimTicket(String ticketId) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.ticketClaim(ticketId),
        data: const <String, dynamic>{},
      );
      return TicketModel.fromJson(res.data ?? const {});
    });
  }

  Future<ServiceResult<T>> _execute<T>(Future<T> Function() body) async {
    try {
      final result = await body();
      return Success(result);
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    } catch (e) {
      return Failure(ApiException(message: e.toString()));
    }
  }
}
