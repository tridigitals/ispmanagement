import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:geolocator/geolocator.dart';

import 'app_config.dart';

/// Auto GPS tracking for technician — sends location every 30 seconds.
class GpsTrackingService {
  GpsTrackingService(this._ref);
  final Ref _ref;
  Timer? _timer;
  bool _running = false;

  void start() {
    if (_running) return;
    _running = true;
    _tick();
    _timer = Timer.periodic(const Duration(seconds: 30), (_) => _tick());
  }

  void stop() {
    _running = false;
    _timer?.cancel();
    _timer = null;
  }

  Future<void> _tick() async {
    try {
      final permission = await Geolocator.checkPermission();
      if (permission == LocationPermission.denied) {
        final requested = await Geolocator.requestPermission();
        if (requested == LocationPermission.denied ||
            requested == LocationPermission.deniedForever) {
          debugPrint('[gps] Location permission denied');
          stop();
          return;
        }
      }
      if (permission == LocationPermission.deniedForever) {
        debugPrint('[gps] Location permission permanently denied');
        stop();
        return;
      }

      final position = await Geolocator.getCurrentPosition(
        locationSettings: const LocationSettings(
          accuracy: LocationAccuracy.high,
          timeLimit: Duration(seconds: 10),
        ),
      );

      final dio = _ref.read(dioProvider);
      await dio.post('/api/technician/location', data: {
        'lat': position.latitude,
        'lng': position.longitude,
        'accuracy': position.accuracy,
      });
      debugPrint('[gps] Location sent: ${position.latitude}, ${position.longitude}');
    } catch (e) {
      debugPrint('[gps] Error: $e');
    }
  }
}

final gpsTrackingServiceProvider = Provider<GpsTrackingService>((ref) {
  return GpsTrackingService(ref);
});
