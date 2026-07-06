import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../models/paginated_response.dart';
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
    String? subscriptionId,
  }) async {
    return _execute(() async {
      final res = await dio.get<dynamic>(
        ApiEndpoints.myInvoices,
        queryParameters: {
          'page': page,
          'per_page': perPage,
          if (status != null) 'status': status,
          if (subscriptionId != null) 'subscription_id': subscriptionId,
        },
      );
      final raw = res.data;
      if (raw is List) {
        final items = raw
            .whereType<Map>()
            .map((e) => InvoiceModel.fromJson(e.cast<String, dynamic>()))
            .toList();
        return PaginatedResponse<InvoiceModel>(
          data: items,
          page: page,
          perPage: perPage,
          total: items.length,
        );
      }
      final json = raw is Map<String, dynamic>
          ? raw
          : (raw is Map ? raw.cast<String, dynamic>() : <String, dynamic>{});
      return PaginatedResponse<InvoiceModel>.fromJson(
        json,
        InvoiceModel.fromJson,
      );
    });
  }

  Future<ServiceResult<InvoiceModel>> getById(String id) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(
        ApiEndpoints.myInvoiceById(id),
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
      // Use backend payment endpoint: /api/payment/invoices/{id}/midtrans or /duitku
      final payEndpoint = method.contains('duitku')
          ? ApiEndpoints.payInvoiceDuitku(invoiceId)
          : ApiEndpoints.payInvoiceMidtrans(invoiceId);
      final res = await dio.post<Map<String, dynamic>>(
        payEndpoint,
        data: {'method': method},
      );
      final url = res.data?['redirect_url'] as String?;
      if (url == null) {
        throw ApiException(message: 'No payment URL returned');
      }
      return url;
    });
  }

  /// Submit payment proof (file ID from storage upload) for an invoice.
  Future<ServiceResult<bool>> submitPaymentProof({
    required String invoiceId,
    required String fileId,
  }) async {
    return _execute(() async {
      await dio.post<dynamic>(
        ApiEndpoints.submitPaymentProof(invoiceId),
        data: {'file_id': fileId, 'proof_file_id': fileId},
      );
      return true;
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
