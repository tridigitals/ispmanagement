import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../auth/auth_token_storage.dart';
import '../models/network_status_model.dart';
import 'auth_service.dart';

class NetworkStatusService {
  NetworkStatusService({required Dio dio, required this.tokenStorage})
      : _dio = dio;
  final Dio _dio;
  final AuthTokenStorage tokenStorage;

  /// Get the current network operational status for the customer's area.
  Future<ServiceResult<NetworkStatusModel>> getStatus() async {
    return _execute(() async {
      final resp = await _dio.get<dynamic>(ApiEndpoints.networkStatus);
      final raw = resp.data;
      final json = raw is Map<String, dynamic>
          ? raw
          : (raw is Map ? raw.cast<String, dynamic>() : <String, dynamic>{});
      return NetworkStatusModel.fromJson(json);
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
