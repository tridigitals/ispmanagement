import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../models/invoice_model.dart';
import 'auth_service.dart';

class InvoiceService {
  InvoiceService({required this.dio});
  final Dio dio;

  Future<ServiceResult<PaginatedResponse<InvoiceModel>>> list({
    int page = 1,
    int perPage = 20,
    String? status,
  }) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(
        ApiEndpoints.myInvoices,
        queryParameters: {
          'page': page,
          'per_page': perPage,
          if (status != null) 'status': status,
        },
      );
      return PaginatedResponse<InvoiceModel>.fromJson(
        res.data ?? const {},
        InvoiceModel.fromJson,
      );
    });
  }

  Future<ServiceResult<InvoiceModel>> getById(String id) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(
        ApiEndpoints.withParam(ApiEndpoints.myInvoiceById, 'id', id),
      );
      return InvoiceModel.fromJson(res.data ?? const {});
    });
  }

  Future<ServiceResult<List<PaymentModel>>> listPayments(String invoiceId) async {
    return _execute(() async {
      final res = await dio.get<List<dynamic>>(
        ApiEndpoints.myPayments,
        queryParameters: {'invoice_id': invoiceId},
      );
      final data = res.data ?? const [];
      return data
          .cast<Map<String, dynamic>>()
          .map(PaymentModel.fromJson)
          .toList();
    });
  }

  /// Trigger a payment flow (returns redirect URL or deep link).
  Future<ServiceResult<String>> initiatePayment({
    required String invoiceId,
    required String method,
  }) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        '/api/portal/payments/initiate',
        data: {'invoice_id': invoiceId, 'method': method},
      );
      final url = res.data?['redirect_url'] as String?;
      if (url == null) {
        throw ApiException(message: 'No payment URL returned');
      }
      return url;
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
