import 'package:dio/dio.dart';

import '../api/api_client.dart';
import 'auth_service.dart';

/// Service for uploading files to the server's storage API.
class StorageService {
  StorageService({required this.dio});
  final Dio dio;

  /// Upload a file and return its file ID.
  ///
  /// [filePath] — absolute path on device.
  /// [fileName] — original filename (e.g. "photo.jpg").
  /// [contentType] — MIME type (e.g. "image/jpeg").
  /// [paymentInvoiceId] — for portal customer upload of payment proof; must
  ///   match an invoice owned by the customer's user account.
  Future<ServiceResult<String>> uploadFile({
    required String filePath,
    required String fileName,
    required String contentType,
    String? ticketId,
    String? paymentInvoiceId,
    bool supportTicketAttachment = false,
  }) async {
    return _execute(() async {
      final formData = FormData.fromMap({
        'file': await MultipartFile.fromFile(
          filePath,
          filename: fileName,
          contentType: DioMediaType.parse(contentType),
        ),
      });

      final queryParameters = <String, dynamic>{
        if (ticketId != null && ticketId.isNotEmpty) 'ticket_id': ticketId,
        if (paymentInvoiceId != null && paymentInvoiceId.isNotEmpty)
          'payment_invoice_id': paymentInvoiceId,
        if (supportTicketAttachment) 'support_ticket_attachment': true,
      };

      final res = await dio.post<Map<String, dynamic>>(
        '/api/storage/upload',
        queryParameters: queryParameters.isEmpty ? null : queryParameters,
        data: formData,
        options: Options(
          headers: {
            'Content-Type': 'multipart/form-data',
          },
        ),
      );

      final data = res.data ?? const {};
      final fileId = data['id'] as String? ?? data['file_id'] as String?;
      if (fileId == null || fileId.isEmpty) {
        throw Exception('Upload succeeded but no file ID returned');
      }
      return fileId;
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
