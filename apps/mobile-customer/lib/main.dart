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

/// Visible error widget for release builds (default ErrorWidget is gray box).
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

Future<void> main() async {
  // Surface widget-tree errors (red screen) instead of blank gray.
  ErrorWidget.builder = _visibleErrorWidget;
  FlutterError.onError = (details) {
    FlutterError.presentError(details);
    if (_sentryDsn.isNotEmpty) {
      Sentry.captureException(details.exception, stackTrace: details.stack);
    }
  };

  // Capture all uncaught Flutter errors → Sentry.
  await runZonedGuarded<Future<void>>(() async {
    WidgetsFlutterBinding.ensureInitialized();

    // ── Phase 1: show visible screen immediately ──
    _runLoadingScreen();

    // ── Phase 2: init services with individual error handling ──
    String? initError;

    // Firebase init (may hang on some devices — timeout after 10s)
    try {
      await Firebase.initializeApp().timeout(
        const Duration(seconds: 10),
        onTimeout: () {
          throw TimeoutException('Firebase.initializeApp() timed out after 10s');
        },
      );
      FirebaseMessaging.onBackgroundMessage(firebaseMessagingBackgroundHandler);
    } catch (e, st) {
      initError = 'Firebase init failed: $e';
      debugPrint(initError);
      debugPrint('$st');
      // Continue without Firebase — app should still work
    }

    // SharedPreferences
    SharedPreferences prefs;
    try {
      prefs = await SharedPreferences.getInstance();
    } catch (e) {
      initError = 'SharedPreferences failed: $e';
      prefs = await SharedPreferences.getInstance(); // fallback
    }

    final onboardingDone = prefs.getBool('onboarding_completed') ?? false;

    if (_sentryDsn.isNotEmpty) {
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
            defaultValue: 'mobile-customer@0.1.0',
          );
          options.sendDefaultPii = false;
          options.beforeSend = (event, hint) {
            if (event.throwable is NetworkException) return null;
            return event;
          };
        },
        appRunner: () => _runApp(prefs, onboardingDone, initError),
      );
    } else {
      _runApp(prefs, onboardingDone, initError);
    }
  }, (error, stack) async {
    if (_sentryDsn.isNotEmpty) {
      await Sentry.captureException(error, stackTrace: stack);
    }
    if (kDebugMode) {
      debugPrint('[Uncaught] $error\n$stack');
    }
  });
}

/// Show a loading screen immediately so the user sees something
/// even if Firebase or other init takes time.
void _runLoadingScreen() {
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

void _runApp(SharedPreferences prefs, bool onboardingDone, String? initError) {
  runApp(
    ProviderScope(
      overrides: [
        onboardingCompletedProvider.overrideWith((ref) => onboardingDone),
        sharedPreferencesProvider.overrideWith((ref) => prefs),
      ],
      child: IspCustomerApp(initError: initError),
    ),
  );
}

class NetworkException implements Exception {
  const NetworkException();
}
