import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../models/notification_model.dart';
import '../models/paginated_response.dart';
import 'auth_service.dart';

class NotificationService {
  NotificationService({required Dio dio}) : _dio = dio;
  final Dio _dio;

  /// List notifications (paginated).
  Future<ServiceResult<PaginatedResponse<NotificationModel>>>
      list({
    int page = 1,
    int perPage = 20,
    bool unreadOnly = false,
  }) async {
    return _execute(() async {
      final resp = await _dio.get<dynamic>(
        ApiEndpoints.notifications,
        queryParameters: {
          'page': page,
          'per_page': perPage,
          if (unreadOnly) 'unread_only': true,
        },
      );
      final raw = resp.data;
      final json = raw is Map<String, dynamic>
          ? raw
          : (raw is Map ? raw.cast<String, dynamic>() : <String, dynamic>{});
      return PaginatedResponse<NotificationModel>.fromJson(
        json,
        (j) => NotificationModel.fromJson(j),
      );
    });
  }

  /// Mark a single notification as read.
  Future<ServiceResult<bool>> markRead(String id) async {
    return _execute(() async {
      await _dio.post<dynamic>(ApiEndpoints.notificationRead(id));
      return true;
    });
  }

  /// Mark all as read.
  Future<ServiceResult<bool>> markAllRead() async {
    return _execute(() async {
      await _dio.post<dynamic>(ApiEndpoints.notificationsReadAll);
      return true;
    });
  }

  /// Unread count (lightweight).
  Future<ServiceResult<int>> unreadCount() async {
    return _execute(() async {
      final resp = await _dio.get<dynamic>(ApiEndpoints.notificationsUnreadCount);
      final raw = resp.data;
      final data = raw is Map<String, dynamic>
          ? raw
          : (raw is Map ? raw.cast<String, dynamic>() : <String, dynamic>{});
      return (data['count'] as num?)?.toInt() ?? 0;
    });
  }

  /// Delete all notifications for the current user.
  Future<ServiceResult<bool>> deleteAll() async {
    return _execute(() async {
      await _dio.delete<dynamic>(ApiEndpoints.notifications);
      return true;
    });
  }

  /// Delete a single notification by ID.
  Future<ServiceResult<bool>> delete(String id) async {
    return _execute(() async {
      await _dio.delete<dynamic>('${ApiEndpoints.notifications}/$id');
      return true;
    });
  }

  Future<ServiceResult<T>> _execute<T extends Object>(Future<T> Function() body) async {
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
