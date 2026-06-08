import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../models/paginated_response.dart';
import '../api/api_endpoints.dart';
import '../models/subscription_model.dart';
import 'auth_service.dart';

class SubscriptionService {
  SubscriptionService({required this.dio});
  final Dio dio;

  /// List the current customer's subscriptions (paginated).
  Future<ServiceResult<PaginatedResponse<SubscriptionModel>>> list({
    int page = 1,
    int perPage = 20,
    String? status,
  }) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(
        ApiEndpoints.mySubscriptions,
        queryParameters: {
          'page': page,
          'per_page': perPage,
          if (status != null) 'status': status,
        },
      );
      return PaginatedResponse<SubscriptionModel>.fromJson(
        res.data ?? const {},
        SubscriptionModel.fromJson,
      );
    });
  }

  /// Get a single subscription by id.
  Future<ServiceResult<SubscriptionModel>> getById(String id) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(
        ApiEndpoints.mySubscriptionById(id),
      );
      return SubscriptionModel.fromJson(res.data ?? const {});
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
