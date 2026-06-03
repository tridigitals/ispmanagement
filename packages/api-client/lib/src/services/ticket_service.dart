import 'package:dio/dio.dart';

import '../api/api_client.dart';
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
        ApiEndpoints.withParam(ApiEndpoints.myTicketById, 'id', id),
      );
      return TicketModel.fromJson(res.data ?? const {});
    });
  }

  Future<ServiceResult<TicketModel>> create({
    required String subject,
    required String description,
    String priority = 'normal',
    String? subscriptionId,
  }) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.createTicket,
        data: {
          'subject': subject,
          'description': description,
          'priority': priority,
          if (subscriptionId != null) 'subscription_id': subscriptionId,
        },
      );
      return TicketModel.fromJson(res.data ?? const {});
    });
  }

  Future<ServiceResult<List<TicketMessageModel>>> listMessages(String ticketId) async {
    return _execute(() async {
      final res = await dio.get<List<dynamic>>(
        ApiEndpoints.withParam(ApiEndpoints.ticketMessages, 'id', ticketId),
      );
      final data = res.data ?? const [];
      return data
          .cast<Map<String, dynamic>>()
          .map(TicketMessageModel.fromJson)
          .toList();
    });
  }

  Future<ServiceResult<TicketMessageModel>> reply({
    required String ticketId,
    required String body,
  }) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.withParam(ApiEndpoints.ticketMessages, 'id', ticketId),
        data: {'body': body},
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
