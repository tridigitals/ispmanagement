import 'package:dio/dio.dart';
import 'package:result_dart/result_dart.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../models/network_status_model.dart';
import 'auth_token_storage.dart';

class NetworkStatusService {
  NetworkStatusService({required Dio dio, required this.tokenStorage})
      : _dio = dio;
  final Dio _dio;
  final AuthTokenStorage tokenStorage;

  /// Get the current network operational status for the customer's area.
  /// Endpoint: `GET /api/portal/network-status`
  Future<Result<NetworkStatusModel, ApiException>> getStatus() async {
    try {
      final resp = await _dio.get<dynamic>(ApiEndpoints.networkStatus);
      return Success(NetworkStatusModel.fromJson(resp.data as Map<String, dynamic>));
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    }
  }
}
