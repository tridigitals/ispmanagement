import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../models/work_order_model.dart';
import 'auth_service.dart';

class WorkOrderService {
  WorkOrderService({required this.dio});
  final Dio dio;

  /// List work orders. Optionally filter by [status] and [assignedTo].
  Future<ServiceResult<List<WorkOrderModel>>> list({
    String? status,
    String? assignedTo,
    bool includeClosed = false,
    int limit = 100,
  }) async {
    return _execute(() async {
      final res = await dio.get<List<dynamic>>(
        ApiEndpoints.workOrders,
        queryParameters: {
          if (status != null) 'status': status,
          if (assignedTo != null) 'assigned_to': assignedTo,
          'include_closed': includeClosed,
          'limit': limit,
        },
      );
      final data = res.data ?? [];
      return data
          .whereType<Map<String, dynamic>>()
          .map(WorkOrderModel.fromJson)
          .toList();
    });
  }

  /// Get a single work order by [id].
  Future<ServiceResult<WorkOrderModel>> getById(String id) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(
        ApiEndpoints.workOrderById(id),
      );
      return WorkOrderModel.fromJson(res.data ?? {});
    });
  }

  /// Claim a pending work order.
  Future<ServiceResult<WorkOrderModel>> claim(String id) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.workOrderClaim(id),
      );
      return WorkOrderModel.fromJson(res.data ?? {});
    });
  }

  /// Start working on an assigned work order.
  Future<ServiceResult<WorkOrderModel>> start(String id) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.workOrderStart(id),
      );
      return WorkOrderModel.fromJson(res.data ?? {});
    });
  }

  /// Complete a work order with optional [notes], [terminalAssetId], [parentAssetId].
  Future<ServiceResult<WorkOrderModel>> complete(
    String id, {
    String? notes,
    String? terminalAssetId,
    String? parentAssetId,
  }) async {
    return _execute(() async {
      final body = <String, dynamic>{};
      if (notes != null) body['notes'] = notes;
      if (terminalAssetId != null) body['terminal_asset_id'] = terminalAssetId;
      if (parentAssetId != null) body['parent_asset_id'] = parentAssetId;

      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.workOrderComplete(id),
        data: body,
      );
      return WorkOrderModel.fromJson(res.data ?? {});
    });
  }

  /// Cancel a work order with optional [notes].
  Future<ServiceResult<WorkOrderModel>> cancel(
    String id, {
    String? notes,
  }) async {
    return _execute(() async {
      final body = <String, dynamic>{};
      if (notes != null) body['notes'] = notes;

      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.workOrderCancel(id),
        data: body,
      );
      return WorkOrderModel.fromJson(res.data ?? {});
    });
  }

  /// Reopen a completed/cancelled work order.
  Future<ServiceResult<WorkOrderModel>> reopen(
    String id, {
    String? notes,
  }) async {
    return _execute(() async {
      final body = <String, dynamic>{};
      if (notes != null) body['notes'] = notes;

      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.workOrderReopen(id),
        data: body,
      );
      return WorkOrderModel.fromJson(res.data ?? {});
    });
  }

  /// Get technician stats (computed from work order list).
  Future<ServiceResult<TechnicianStats>> getStats(String userId) async {
    return _execute(() async {
      // Get all assigned work orders
      final res = await dio.get<List<dynamic>>(
        ApiEndpoints.workOrders,
        queryParameters: {
          'assigned_to': userId,
          'include_closed': true,
          'limit': 1000,
        },
      );
      final data = res.data ?? [];
      final orders =
          data.whereType<Map<String, dynamic>>().map(WorkOrderModel.fromJson).toList();

      final now = DateTime.now();
      final todayStart = DateTime(now.year, now.month, now.day);
      final weekStart = todayStart.subtract(Duration(days: now.weekday - 1));

      int completed = 0;
      int inProgress = 0;
      int pending = 0;
      int cancelled = 0;
      int completedToday = 0;
      int completedThisWeek = 0;
      double totalHours = 0;
      int completedWithTime = 0;

      for (final wo in orders) {
        switch (wo.status) {
          case 'completed':
            completed++;
            if (wo.completedAt != null) {
              if (wo.completedAt!.isAfter(todayStart)) completedToday++;
              if (wo.completedAt!.isAfter(weekStart)) completedThisWeek++;
              if (wo.createdAt != null) {
                totalHours +=
                    wo.completedAt!.difference(wo.createdAt!).inMinutes / 60.0;
                completedWithTime++;
              }
            }
            break;
          case 'in_progress':
            inProgress++;
            break;
          case 'pending':
          case 'assigned':
            pending++;
            break;
          case 'cancelled':
            cancelled++;
            break;
        }
      }

      return TechnicianStats(
        totalAssigned: orders.length,
        completed: completed,
        inProgress: inProgress,
        pending: pending,
        cancelled: cancelled,
        avgCompletionHours: completedWithTime > 0
            ? totalHours / completedWithTime
            : null,
        completedToday: completedToday,
        completedThisWeek: completedThisWeek,
      );
    });
  }

  Future<ServiceResult<T>> _execute<T>(Future<T> Function() fn) async {
    try {
      final result = await fn();
      return Success(result);
    } on DioException catch (e) {
      return Failure(ApiException(message: _mapDioError(e)));
    } catch (e) {
      return Failure(ApiException(message: e.toString()));
    }
  }

  String _mapDioError(DioException e) {
    switch (e.type) {
      case DioExceptionType.connectionTimeout:
      case DioExceptionType.sendTimeout:
      case DioExceptionType.receiveTimeout:
        return 'Koneksi timeout. Coba lagi.';
      case DioExceptionType.connectionError:
        return 'Tidak bisa terhubung ke server.';
      case DioExceptionType.badResponse:
        final statusCode = e.response?.statusCode;
        final data = e.response?.data;
        if (data is Map<String, dynamic> && data.containsKey('error')) {
          return data['error'].toString();
        }
        return 'Error $statusCode: ${e.message}';
      default:
        return e.message ?? 'Terjadi kesalahan.';
    }
  }
}
