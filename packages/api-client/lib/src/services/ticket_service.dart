import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../models/paginated_response.dart';
import '../api/api_endpoints.dart';
import '../models/ticket_model.dart';
import 'auth_service.dart';

class TicketService {
  TicketService({required this.dio});
  final Dio dio;

  Future<ServiceResult<PaginatedResponse<TicketModel>>> list({
    int page = 1,
    int perPage = 20,
    String? status,
  }) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(
        ApiEndpoints.myTickets,
        queryParameters: {
          'page': page,
          'per_page': perPage,
          if (status != null) 'status': status,
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
    List<String>? attachmentIds,
  }) async {
    return _execute(() async {
      final body = <String, dynamic>{
        'subject': subject,
        'message': message,
        'priority': priority,
      };
      if (attachmentIds != null && attachmentIds.isNotEmpty) {
        body['attachment_ids'] = attachmentIds;
      }
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.createTicket,
        data: body,
      );
      return TicketModel.fromJson(res.data ?? const {});
    });
  }

  /// Fetch messages for a ticket.
  /// [currentUserId] is used to determine which messages are from staff vs customer.
  Future<ServiceResult<List<TicketMessageModel>>> listMessages(
    String ticketId, {
    String? currentUserId,
  }) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(
        ApiEndpoints.myTicketById(ticketId),
      );
      final data = res.data ?? const {};
      final messages = data['messages'];
      if (messages is List) {
        return messages
            .whereType<Map<String, dynamic>>()
            .map((json) => currentUserId != null
                ? TicketMessageModel.fromTicketJson(json, currentUserId)
                : TicketMessageModel.fromJson(json))
            .toList();
      }
      return <TicketMessageModel>[];
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
