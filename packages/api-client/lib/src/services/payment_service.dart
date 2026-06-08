import 'package:api_client/api_client.dart';
import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../models/payment_model.dart';
import 'auth_service.dart';

class PaymentService {
  PaymentService({required Dio dio, StorageService? storageService})
      : _dio = dio,
        _storageService = storageService;
  final Dio _dio;
  final StorageService? _storageService;

  /// Initiate payment via Midtrans.
  /// Returns the redirect URL string.
  Future<ServiceResult<String>> initiateMidtrans(String invoiceId) async {
    return _execute(() async {
      final resp = await _dio.post<dynamic>(
        ApiEndpoints.payInvoiceMidtrans(invoiceId),
        data: {},
      );
      final raw = resp.data;
      // Backend returns the redirect URL as a plain string or {redirect_url: "..."}
      if (raw is String) return raw;
      if (raw is Map<String, dynamic>) {
        return (raw['redirect_url'] ?? raw['url'] ?? raw.toString()).toString();
      }
      return raw.toString();
    });
  }

  /// Initiate payment via Duitku.
  /// Returns the redirect URL string.
  Future<ServiceResult<String>> initiateDuitku(String invoiceId) async {
    return _execute(() async {
      final resp = await _dio.post<dynamic>(
        ApiEndpoints.payInvoiceDuitku(invoiceId),
        data: {},
      );
      final raw = resp.data;
      if (raw is String) return raw;
      if (raw is Map<String, dynamic>) {
        return (raw['redirect_url'] ?? raw['url'] ?? raw.toString()).toString();
      }
      return raw.toString();
    });
  }

  /// Submit payment proof (receipt/bank transfer screenshot) for an invoice.
  ///
  /// Uploads the file via [StorageService] then submits the file ID to the
  /// payment proof endpoint.
  Future<ServiceResult<bool>> submitPaymentProof({
    required String invoiceId,
    required String filePath,
    required String fileName,
    required String contentType,
  }) async {
    return _execute(() async {
      if (_storageService == null) {
        throw Exception('StorageService not available');
      }
      // First upload the file
      final uploadResult = await _storageService!.uploadFile(
        filePath: filePath,
        fileName: fileName,
        contentType: contentType,
      );
      final fileId = uploadResult.getOrThrow();

      // Then submit the proof with the file ID
      await _dio.post<dynamic>(
        ApiEndpoints.submitPaymentProof(invoiceId),
        data: {'file_id': fileId},
      );
      return true;
    });
  }

  /// Get available payment channels for an invoice.
  Future<ServiceResult<List<PaymentChannel>>> paymentChannels(
    String invoiceId,
  ) async {
    return _execute(() async {
      final resp = await _dio.get<dynamic>(
        ApiEndpoints.paymentChannels(invoiceId),
      );
      final raw = resp.data;
      if (raw is List) {
        return raw
            .whereType<Map<String, dynamic>>()
            .map(PaymentChannel.fromJson)
            .toList();
      }
      if (raw is Map<String, dynamic>) {
        final data = raw['data'] ?? raw['channels'] ?? raw['methods'];
        if (data is List) {
          return data
              .whereType<Map<String, dynamic>>()
              .map(PaymentChannel.fromJson)
              .toList();
        }
      }
      return <PaymentChannel>[];
    });
  }

  /// Poll a transaction's current status.
  Future<ServiceResult<PaymentTransaction>> getTransaction(
    String transactionId,
  ) async {
    return _execute(() async {
      final resp = await _dio.get<dynamic>(
        ApiEndpoints.paymentById(transactionId),
      );
      final raw = resp.data;
      final json = raw is Map<String, dynamic>
          ? raw
          : (raw is Map ? raw.cast<String, dynamic>() : <String, dynamic>{});
      return PaymentTransaction.fromJson(json);
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
