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

Future<void> main() async {
  // Capture all uncaught Flutter errors → Sentry.
  await runZonedGuarded<Future<void>>(() async {
    WidgetsFlutterBinding.ensureInitialized();
    // Firebase init.
    await Firebase.initializeApp();
    // Register FCM background handler.
    FirebaseMessaging.onBackgroundMessage(firebaseMessagingBackgroundHandler);
    final prefs = await SharedPreferences.getInstance();
    final onboardingDone = prefs.getBool('onboarding_completed') ?? false;

    if (_sentryDsn.isNotEmpty) {
      await SentryFlutter.init(
        (options) {
          options.dsn = _sentryDsn;
          options.tracesSampleRate = 0.2; // 20% perf traces
          options.profilesSampleRate = 0.1;
          options.environment = const String.fromEnvironment(
            'SENTRY_ENV',
            defaultValue: 'production',
          );
          options.release = const String.fromEnvironment(
            'SENTRY_RELEASE',
            defaultValue: 'mobile-customer@0.1.0',
          );
          // Don't send PII (email, IP) by default
          options.sendDefaultPii = false;
          // Drop noisy errors
          options.beforeSend = (event, hint) {
            if (event.throwable is NetworkException) return null;
            return event;
          };
        },
        appRunner: () => _runApp(prefs, onboardingDone),
      );
    } else {
      _runApp(prefs, onboardingDone);
    }
  }, (error, stack) async {
    // Final safety net — any uncaught async error lands here.
    if (_sentryDsn.isNotEmpty) {
      await Sentry.captureException(error, stackTrace: stack);
    }
    if (kDebugMode) {
      debugPrint('[Uncaught] $error\n$stack');
    }
  });
}

void _runApp(SharedPreferences prefs, bool onboardingDone) {
  runApp(
    ProviderScope(
      overrides: [
        onboardingCompletedProvider.overrideWith((ref) => onboardingDone),
        sharedPreferencesProvider.overrideWith((ref) => prefs),
      ],
      child: const IspCustomerApp(),
    ),
  );
}

class NetworkException implements Exception {
  const NetworkException();
}
