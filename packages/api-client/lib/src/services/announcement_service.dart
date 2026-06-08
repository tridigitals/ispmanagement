import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../models/announcement_model.dart';
import '../models/paginated_response.dart';
import 'auth_service.dart';

class AnnouncementService {
  AnnouncementService(this._dio);
  final Dio _dio;

  Future<ServiceResult<PaginatedResponse<AnnouncementModel>>> listRecent({
    int page = 1,
    int perPage = 20,
  }) async {
    return _execute(() async {
      final res = await _dio.get<Map<String, dynamic>>(
        ApiEndpoints.announcementsRecent,
        queryParameters: {'page': page, 'per_page': perPage},
      );
      return PaginatedResponse.fromJson(
        res.data ?? const {},
        AnnouncementModel.fromJson,
      );
    });
  }

  Future<ServiceResult<AnnouncementModel>> getById(String id) async {
    return _execute(() async {
      final res = await _dio.get<Map<String, dynamic>>(
        '/api/announcements/$id',
      );
      return AnnouncementModel.fromJson(res.data ?? const {});
    });
  }

  Future<ServiceResult<List<AnnouncementModel>>> getActive() async {
    return _execute(() async {
      final res = await _dio.get<List<dynamic>>(
        ApiEndpoints.announcementsActive,
      );
      return (res.data ?? [])
          .map((e) => AnnouncementModel.fromJson(e as Map<String, dynamic>))
          .toList();
    });
  }

  Future<ServiceResult<void>> dismiss(String id) async {
    return _execute(() async {
      await _dio.post<void>(
        ApiEndpoints.announcementDismiss(id),
      );
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
