import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../models/network_asset_model.dart';
import 'auth_service.dart';

/// Service for fetching/creating network assets (ONT/ONU).
class NetworkAssetService {
  NetworkAssetService({required this.dio});
  final Dio dio;

  /// List terminal assets (ONT/ONU) for a specific customer.
  Future<ServiceResult<List<NetworkAssetListItemModel>>> listByCustomer(
    String customerId,
  ) async {
    return _execute(() async {
      final res = await dio.get<List<dynamic>>(
        ApiEndpoints.networkAssetsByCustomer(customerId),
      );
      return (res.data ?? [])
          .whereType<Map<String, dynamic>>()
          .map(NetworkAssetListItemModel.fromJson)
          .where((a) => a.assetType == 'ont' || a.assetType == 'onu')
          .toList();
    });
  }

  /// Create a new terminal asset (ONT/ONU) for a customer.
  Future<ServiceResult<NetworkAssetListItemModel>> create({
    required String assetType,
    required String name,
    String? customerId,
    String? serialNumber,
    String? vendor,
    String? model,
    String? notes,
  }) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.networkAssets,
        data: {
          'asset_group': 'terminal',
          'asset_type': assetType,
          'name': name,
          if (customerId != null) 'customer_id': customerId,
          if (serialNumber != null) 'serial_number': serialNumber,
          if (vendor != null) 'vendor': vendor,
          if (model != null) 'model': model,
          if (notes != null) 'notes': notes,
        },
      );
      return NetworkAssetListItemModel.fromJson(res.data!);
    });
  }

  Future<ServiceResult<T>> _execute<T>(
    Future<T> Function() body,
  ) async {
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
