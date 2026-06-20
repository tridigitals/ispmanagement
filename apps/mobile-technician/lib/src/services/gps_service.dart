import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:geolocator/geolocator.dart';

import 'service_providers.dart';

/// GPS tracking service for field technicians.
/// Periodically captures location and POSTs to backend.
/// Auto-pauses when permission is denied — silent fail (admin loses tracking).
class GpsTrackingService {
  GpsTrackingService(this._ref);
  final Ref _ref;

  Timer? _timer;
  bool _running = false;
  bool _permissionDenied = false;

  bool get isRunning => _running;
  bool get permissionDenied => _permissionDenied;

  /// Start the periodic GPS tracker.
  /// Idempotent — calling twice is a no-op.
  Future<void> start() async {
    if (_running) return;
    _running = true;
    _status('start() called');

    // Step 1: check permission
    if (!await _ensurePermission()) {
      _permissionDenied = true;
      _running = false;
      _status('⚠️ permission denied — GPS disabled');
      return;
    }

    // Step 2: capture initial position + start timer
    _timer = Timer.periodic(
      const Duration(minutes: 2),
      (_) => _captureAndSend(),
    );
    _status('Timer started — first ping in ~5s');
    // Don't fire immediately on start — give user 5s grace.
    Timer(const Duration(seconds: 5), _captureAndSend);
  }

  /// Stop the tracker. Safe to call when not running.
  void stop() {
    _timer?.cancel();
    _timer = null;
    _running = false;
    _status('stop()');
  }

  /// Manual single capture (e.g. for "send my location now" button).
  Future<void> captureNow() => _captureAndSend();

  Future<bool> _ensurePermission() async {
    try {
      // Service enabled?
      final serviceEnabled = await Geolocator.isLocationServiceEnabled();
      if (!serviceEnabled) {
        _status('⚠️ location service disabled on device');
        return false;
      }

      // Current permission status
      var permission = await Geolocator.checkPermission();
      if (permission == LocationPermission.denied) {
        permission = await Geolocator.requestPermission();
      }
      if (permission == LocationPermission.denied ||
          permission == LocationPermission.deniedForever) {
        return false;
      }
      return true;
    } catch (e) {
      _status('❌ permission check failed: $e');
      return false;
    }
  }

  Future<void> _captureAndSend() async {
    try {
      final position = await Geolocator.getCurrentPosition(
        locationSettings: const LocationSettings(
          accuracy: LocationAccuracy.high,
          timeLimit: Duration(seconds: 10),
        ),
      );
      _status(
        '📍 Fix: ${position.latitude.toStringAsFixed(5)}, '
        '${position.longitude.toStringAsFixed(5)} '
        '(±${position.accuracy.toStringAsFixed(0)}m)',
      );

      final svc = _ref.read(locationServiceProvider);
      final res = await svc.recordLocation(
        latitude: position.latitude,
        longitude: position.longitude,
        accuracy: position.accuracy,
        altitude: position.altitude,
        bearing: position.heading,
        speed: position.speed,
        capturedAt: position.timestamp,
      );
      res.fold(
        (_) => _status('✅ Location sent to server'),
        (err) => _status('❌ Failed to send: ${err.message}'),
      );
    } on TimeoutException {
      _status('⚠️ getCurrentPosition timeout');
    } catch (e) {
      _status('❌ capture error: $e');
    }
  }

  void _status(String msg) {
    if (kDebugMode) debugPrint('[GPS] $msg');
  }
}

final gpsTrackingServiceProvider = Provider<GpsTrackingService>((ref) {
  final svc = GpsTrackingService(ref);
  ref.onDispose(svc.stop);
  return svc;
});