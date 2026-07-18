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

  /// Backend returns a Snap **token** (UUID, no scheme). Web uses `snap.pay(token)`.
  /// Mobile WebView needs a full URL → Midtrans vtweb page.
  static String resolveMidtransPaymentUrl(
    String tokenOrUrl, {
    bool isProduction = false,
  }) {
    final s = tokenOrUrl.trim();
    if (s.isEmpty) return s;
    if (s.contains('://')) return s;
    final base = isProduction
        ? 'https://app.midtrans.com/snap/v2/vtweb/'
        : 'https://app.sandbox.midtrans.com/snap/v2/vtweb/';
    return '$base$s';
  }

  static String _extractPaymentValue(dynamic raw) {
    if (raw is String) return raw;
    if (raw is Map) {
      final map = raw is Map<String, dynamic>
          ? raw
          : raw.map((k, v) => MapEntry(k.toString(), v));
      final redirect = map['redirect_url'] ?? map['url'] ?? map['redirectUrl'];
      if (redirect != null && redirect.toString().trim().isNotEmpty) {
        return redirect.toString();
      }
      final token = map['token'];
      if (token != null && token.toString().trim().isNotEmpty) {
        return token.toString();
      }
    }
    return raw?.toString() ?? '';
  }

  /// Initiate payment via Midtrans.
  /// Returns a loadable payment URL (token auto-converted to Snap vtweb).
  Future<ServiceResult<String>> initiateMidtrans(
    String invoiceId, {
    bool isProduction = false,
  }) async {
    return _execute(() async {
      final resp = await _dio.post<dynamic>(
        ApiEndpoints.payInvoiceMidtrans(invoiceId),
        data: {},
      );
      return resolveMidtransPaymentUrl(
        _extractPaymentValue(resp.data),
        isProduction: isProduction,
      );
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
      final value = _extractPaymentValue(resp.data);
      if (!value.contains('://') && value.isNotEmpty) {
        throw ApiException(
          message: 'Duitku tidak mengembalikan URL pembayaran',
        );
      }
      return value;
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
      // First upload the file (with payment_invoice_id query param so the
      // backend can verify portal customer ownership of the invoice)
      final uploadResult = await _storageService!.uploadFile(
        filePath: filePath,
        fileName: fileName,
        contentType: contentType,
        paymentInvoiceId: invoiceId,
      );
      final fileId = uploadResult.getOrThrow();

      // Then submit the proof with the file ID as file_path
      // (backend's SubmitPaymentProofBody expects camelCase `filePath`)
      await _dio.post<dynamic>(
        ApiEndpoints.submitPaymentProof(invoiceId),
        data: {'filePath': fileId},
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
