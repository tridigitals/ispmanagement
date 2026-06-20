import 'package:dio/dio.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';

/// Records a single GPS ping from the technician's phone.
/// Backend: src-tauri/src/commands/technician_location.rs::record_technician_location
class LocationService {
  LocationService({required this.dio});
  final Dio dio;

  /// Send a single GPS fix to the backend.
  /// Fire-and-forget — failures are silent (admin loses 1 ping, not a big deal).
  Future<ServiceResult<RecordedLocation>> recordLocation({
    required double latitude,
    required double longitude,
    double? accuracy,
    double? altitude,
    double? bearing,
    double? speed,
    DateTime? capturedAt,
    int? batteryLevel,
  }) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.technicianLocations,
        data: {
          'latitude': latitude,
          'longitude': longitude,
          if (accuracy != null) 'accuracy': accuracy,
          if (altitude != null) 'altitude': altitude,
          if (bearing != null) 'bearing': bearing,
          if (speed != null) 'speed': speed,
          'captured_at': (capturedAt ?? DateTime.now()).toUtc().toIso8601String(),
          if (batteryLevel != null) 'battery_level': batteryLevel,
        },
      );
      final data = res.data ?? const {};
      return RecordedLocation.fromJson(data);
    });
  }

  /// Fetch the latest known location for a single technician.
  /// Returns null if no location has been recorded yet.
  Future<ServiceResult<RecordedLocation?>> getLatest(String technicianId) async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>?>(
        ApiEndpoints.latestTechnicianLocation(technicianId),
      );
      final data = res.data;
      if (data == null) return null;
      return RecordedLocation.fromJson(data);
    });
  }

  Future<ServiceResult<T>> _execute<T>(Future<T> Function() fn) async {
    try {
      final result = await fn().timeout(
        const Duration(seconds: 10),
        onTimeout: () => throw Exception('Location request timeout'),
      );
      return Success(result);
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    } catch (e) {
      return Failure(ApiException(message: e.toString()));
    }
  }
}

/// Response shape from POST /api/technician/locations or
/// GET /api/technician/locations/:id/latest
class RecordedLocation {
  RecordedLocation({
    required this.id,
    required this.technicianId,
    required this.capturedAt,
    this.latitude,
    this.longitude,
    this.accuracy,
  });

  final String id;
  final String technicianId;
  final DateTime capturedAt;
  final double? latitude;
  final double? longitude;
  final double? accuracy;

  factory RecordedLocation.fromJson(Map<String, dynamic> json) {
    return RecordedLocation(
      id: json['id'] as String? ?? '',
      technicianId: json['technician_id'] as String? ?? '',
      capturedAt: DateTime.parse(
        json['captured_at'] as String? ?? DateTime.now().toIso8601String(),
      ),
      latitude: (json['latitude'] as num?)?.toDouble(),
      longitude: (json['longitude'] as num?)?.toDouble(),
      accuracy: (json['accuracy'] as num?)?.toDouble(),
    );
  }
}