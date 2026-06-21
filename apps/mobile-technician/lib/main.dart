import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'src/app.dart';
import 'src/services/fcm_service.dart';
import 'src/services/settings_providers.dart';
import 'src/services/missing_providers.dart';

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

/// Entry point — shows loading screen immediately, then initializes
/// services in background. Timer safety net ensures app always starts.
void main() {
  // Surface widget-tree errors instead of blank gray.
  ErrorWidget.builder = _visibleErrorWidget;

  WidgetsFlutterBinding.ensureInitialized();

  // Phase 1: Show loading screen IMMEDIATELY (synchronous).
  _showLoadingScreen();

  // Phase 2: Safety net — force app start after 8 seconds no matter what.
  Timer(const Duration(seconds: 8), () {
    if (!_appStarted) {
      debugPrint('[safety] 8s elapsed — forcing app start without services');
      _startApp(null, false, initError: 'Initialization timeout (8s)');
    }
  });

  // Phase 3: Init services in background (fire-and-forget).
  _initServices();
}

/// Shows a minimal loading screen while services initialize.
void _showLoadingScreen() {
  runApp(
    const MaterialApp(
      debugShowCheckedModeBanner: false,
      home: Scaffold(
        backgroundColor: Color(0xFF08090D),
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              CircularProgressIndicator(color: Color(0xFF1565C0)),
              SizedBox(height: 16),
              Text(
                'Memuat...',
                style: TextStyle(color: Colors.white70, fontSize: 14),
              ),
            ],
          ),
        ),
      ),
    ),
  );
}

/// Initialize Firebase, SharedPreferences.
/// On ANY failure, starts the app with error info.
Future<void> _initServices() async {
  String? initError;

  // ── Firebase (5s timeout) ──
  try {
    await Firebase.initializeApp().timeout(
      const Duration(seconds: 5),
      onTimeout: () {
        throw TimeoutException('Firebase timeout (5s)');
      },
    );
    FirebaseMessaging.onBackgroundMessage(firebaseMessagingBackgroundHandler);
    debugPrint('[init] Firebase OK');
  } catch (e, st) {
    initError = 'Firebase init failed: $e';
    debugPrint('[init] $initError');
    debugPrint('$st');
  }

  // ── SharedPreferences ──
  SharedPreferences? prefs;
  try {
    prefs = await SharedPreferences.getInstance();
    debugPrint('[init] SharedPreferences OK');
  } catch (e) {
    initError = (initError != null)
        ? '$initError\nPrefs failed: $e'
        : 'Prefs failed: $e';
    debugPrint('[init] SharedPreferences failed: $e');
  }

  final onboardingDone = prefs?.getBool('onboarding_completed') ?? false;

  _startApp(prefs, onboardingDone, initError: initError);
}

/// Start the real app. Safe to call multiple times (idempotent).
void _startApp(
  SharedPreferences? prefs,
  bool onboardingDone, {
  String? initError,
}) {
  if (_appStarted) return;
  _appStarted = true;
  debugPrint(
      '[start] App starting — onboardingDone=$onboardingDone, error=$initError');

  final List<Override> overrides = [
    onboardingCompletedProvider.overrideWith((ref) => onboardingDone),
  ];

  if (prefs != null) {
    overrides.add(sharedPreferencesProvider.overrideWith((ref) => prefs));
  }

  runApp(
    ProviderScope(
      overrides: overrides,
      child: IspTechnicianApp(initError: initError),
    ),
  );
}
