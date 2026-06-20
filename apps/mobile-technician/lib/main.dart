import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';

import 'src/app.dart';

/// Track whether real app has started (prevents double-start).
bool _appStarted = false;

/// Visible error widget for release builds.
Widget _visibleErrorWidget(FlutterErrorDetails details) {
  return Container(
    color: const Color(0xFFFFEBEE),
    padding: const EdgeInsets.all(16),
    alignment: Alignment.center,
    child: SingleChildScrollView(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.error_outline, color: Color(0xFFC62828), size: 48),
          const SizedBox(height: 12),
          const Text(
            'Terjadi kesalahan render',
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w600,
              color: Color(0xFFB71C1C),
            ),
          ),
          const SizedBox(height: 8),
          Text(
            details.exceptionAsString(),
            textAlign: TextAlign.center,
            style: const TextStyle(fontSize: 12, color: Color(0xFF424242)),
          ),
        ],
      ),
    ),
  );
}

/// Background message handler — must be a top-level function.
@pragma('vm:entry-point')
Future<void> _firebaseBackgroundHandler(RemoteMessage message) async {
  await Firebase.initializeApp();
}

Future<void> main() async {
  if (_appStarted) return;
  _appStarted = true;

  // Catch all uncaught Flutter framework errors so the UI shows something
  // useful instead of an eternal red screen in release mode.
  ErrorWidget.builder = (FlutterErrorDetails details) {
    return _visibleErrorWidget(details);
  };

  String? initError;
  await runZonedGuarded<Future<void>>(() async {
    WidgetsFlutterBinding.ensureInitialized();

    // Firebase — best-effort. App still runs without it (FCM disabled).
    try {
      await Firebase.initializeApp();
      FirebaseMessaging.onBackgroundMessage(_firebaseBackgroundHandler);
    } catch (e) {
      initError = 'Firebase init gagal: $e';
      debugPrint('[main] $initError');
    }

    runApp(ProviderScope(child: IspTechnicianApp(initError: initError)));
  }, (error, stack) {
    debugPrint('[main] uncaught zone error: $error\n$stack');
  });
}