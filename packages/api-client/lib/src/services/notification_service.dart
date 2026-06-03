import 'package:dio/dio.dart';
import 'package:result_dart/result_dart.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../models/notification_model.dart';

class NotificationService {
  NotificationService({required Dio dio}) : _dio = dio;
  final Dio _dio;

  /// List notifications (paginated).
  Future<Result<PaginatedResponse<NotificationModel>, ApiException>> list({
    int page = 1,
    int perPage = 20,
    bool unreadOnly = false,
  }) async {
    try {
      final resp = await _dio.get<dynamic>(
        ApiEndpoints.notifications,
        queryParameters: {
          'page': page,
          'per_page': perPage,
          if (unreadOnly) 'unread_only': true,
        },
      );
      return Success(
        PaginatedResponse<NotificationModel>.fromJson(
          resp.data as Map<String, dynamic>,
          (j) => NotificationModel.fromJson(j),
        ),
      );
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    }
  }

  /// Mark a single notification as read.
  Future<Result<void, ApiException>> markRead(String id) async {
    try {
      await _dio.post<dynamic>(ApiEndpoints.notificationRead(id));
      return const Success(null);
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    }
  }

  /// Mark all as read.
  Future<Result<void, ApiException>> markAllRead() async {
    try {
      await _dio.post<dynamic>(ApiEndpoints.notificationsReadAll);
      return const Success(null);
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    }
  }

  /// Unread count (lightweight).
  Future<Result<int, ApiException>> unreadCount() async {
    try {
      final resp = await _dio.get<dynamic>(ApiEndpoints.notificationsUnreadCount);
      final data = resp.data as Map<String, dynamic>;
      return Success((data['count'] as num).toInt());
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    }
  }
}
