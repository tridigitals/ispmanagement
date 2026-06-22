import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:sentry_flutter/sentry_flutter.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'src/app.dart';
import 'src/services/fcm_service.dart';
import 'src/services/settings_providers.dart';
import 'src/services/missing_providers.dart';

/// Sentry DSN — pass via --dart-define=SENTRY_DSN=...
/// Empty string = Sentry disabled (dev / local builds).
const _sentryDsn = String.fromEnvironment('SENTRY_DSN', defaultValue: '');

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
  FlutterError.onError = (details) {
    FlutterError.presentError(details);
    if (_sentryDsn.isNotEmpty) {
      Sentry.captureException(details.exception, stackTrace: details.stack);
    }
  };

  WidgetsFlutterBinding.ensureInitialized();

  // Phase 1: Show loading screen IMMEDIATELY (synchronous).
  _showLoadingScreen();

  // Phase 2: Safety net — force app start after 8 seconds no matter what.
  // This Timer is scheduled BEFORE any async work, so it fires even if
  // Firebase or SharedPreferences hang the event loop.
  Timer(const Duration(seconds: 8), () {
    if (!_appStarted) {
      debugPrint('[safety] 8s elapsed — forcing app start without services');
      _startApp(null, false, initError: 'Initialization timeout (8s)');
    }
  });

  // Phase 3: Init services in background (fire-and-forget).
  // Errors are caught locally; if anything fails, app still starts.
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
              CircularProgressIndicator(color: Color(0xFF8B9CFF)),
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

/// Initialize Firebase, SharedPreferences, Sentry.
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
    // Register FCM background handler
    FirebaseMessaging.onBackgroundMessage(firebaseMessagingBackgroundHandler);
    debugPrint('[init] Firebase OK');
  } catch (e, st) {
    initError = 'Firebase init failed: $e';
    debugPrint('[init] $initError');
    debugPrint('$st');
    // Continue without Firebase — app should still work
  }

  // ── SharedPreferences ──
  SharedPreferences? prefs;
  try {
    prefs = await SharedPreferences.getInstance();
    debugPrint('[init] SharedPreferences OK');
  } catch (e) {
    initError = (initError != null) ? '$initError\nPrefs failed: $e' : 'Prefs failed: $e';
    debugPrint('[init] SharedPreferences failed: $e');
    // Continue without prefs — defaults will be used
  }

  final onboardingDone = prefs?.getBool('onboarding_completed') ?? false;

  // ── Sentry (optional) ──
  if (_sentryDsn.isNotEmpty) {
    try {
      await SentryFlutter.init(
        (options) {
          options.dsn = _sentryDsn;
          options.tracesSampleRate = 0.2;
          options.profilesSampleRate = 0.1;
          options.environment = const String.fromEnvironment(
            'SENTRY_ENV',
            defaultValue: 'production',
          );
          options.release = const String.fromEnvironment(
            'SENTRY_RELEASE',
            defaultValue: 'mobile-technician@0.2.0',
          );
          options.sendDefaultPii = false;
          options.beforeSend = (event, hint) {
            if (event.throwable is NetworkException) return null;
            return event;
          };
        },
        appRunner: () => _startApp(prefs, onboardingDone, initError: initError),
      );
    } catch (e) {
      debugPrint('[init] Sentry init failed: $e');
      _startApp(prefs, onboardingDone, initError: initError);
    }
  } else {
    _startApp(prefs, onboardingDone, initError: initError);
  }
}

/// Start the real app. Safe to call multiple times (idempotent).
void _startApp(
  SharedPreferences? prefs,
  bool onboardingDone, {
  String? initError,
}) {
  if (_appStarted) return; // Already started (by timer or by _initServices)
  _appStarted = true;
  debugPrint('[start] App starting — onboardingDone=$onboardingDone, error=$initError');

  final List<Override> overrides = [
    onboardingCompletedProvider.overrideWith((ref) => onboardingDone),
  ];

  // Only override SharedPreferences if we actually got an instance.
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

class NetworkException implements Exception {
  const NetworkException();
}
