import 'package:dio/dio.dart';
import 'package:result_dart/result_dart.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../models/invoice_model.dart';
import '../models/payment_model.dart';

class PaymentService {
  PaymentService({required Dio dio}) : _dio = dio;
  final Dio _dio;

  /// Get available payment channels for an invoice.
  /// Endpoint: `GET /api/portal/invoices/:id/payment-channels`
  Future<Result<List<PaymentChannel>, ApiException>> getChannels(
    String invoiceId,
  ) async {
    try {
      final resp = await _dio.get<dynamic>(
        ApiEndpoints.paymentChannels(invoiceId),
      );
      final list = (resp.data as List<dynamic>)
          .map((j) => PaymentChannel.fromJson(j as Map<String, dynamic>))
          .toList(growable: false);
      return Success(list);
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    }
  }

  /// Create a new payment transaction.
  /// Endpoint: `POST /api/portal/invoices/:id/pay`
  Future<Result<PaymentTransaction, ApiException>> createTransaction({
    required String invoiceId,
    required String paymentChannelCode,
  }) async {
    try {
      final resp = await _dio.post<dynamic>(
        ApiEndpoints.payInvoice(invoiceId),
        data: {'payment_channel': paymentChannelCode},
      );
      return Success(
        PaymentTransaction.fromJson(resp.data as Map<String, dynamic>),
      );
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    }
  }

  /// Poll a transaction's current status (used for VA / QRIS while waiting).
  /// Endpoint: `GET /api/portal/payments/:transactionId`
  Future<Result<PaymentTransaction, ApiException>> getTransaction(
    String transactionId,
  ) async {
    try {
      final resp = await _dio.get<dynamic>(
        ApiEndpoints.paymentById(transactionId),
      );
      return Success(
        PaymentTransaction.fromJson(resp.data as Map<String, dynamic>),
      );
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    }
  }
}
