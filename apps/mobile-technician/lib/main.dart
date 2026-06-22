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

  // Phase 2: SharedPreferences FIRST — required for auth session.
  // This MUST complete before the safety timer fires, otherwise the
  // timer starts the app with prefs=null and onboardingDone=false,
  // permanently overriding the providers and breaking session persistence.
  SharedPreferences.getInstance().then((prefs) {
    _onPrefsReady(prefs);
  }).catchError((e) {
    debugPrint('[init] SharedPreferences failed: $e — starting without prefs');
    _onPrefsReady(null);
  });

  // Phase 3: Safety net — force app start after 8 seconds no matter what.
  // Only fires if _onPrefsReady() hasn't run yet (e.g., both Firebase AND
  // SharedPreferences hung). Prefs may still be null here — app starts
  // with defaults, user logs in fresh.
  Timer(const Duration(seconds: 8), () {
    if (!_appStarted) {
      debugPrint('[safety] 8s elapsed — forcing app start (prefs may be null)');
      _startApp(null, false, initError: 'Initialization timeout (8s)');
    }
  });

  // Phase 4: Firebase in background (fire-and-forget — non-critical).
  // Errors are caught; app still starts without FCM.
  _initFirebase();
}

/// Called once SharedPreferences is ready (or null on failure).
Future<void> _onPrefsReady(SharedPreferences? prefs) async {
  if (_appStarted) return; // Safety timer already started app
  debugPrint('[init] SharedPreferences ${prefs != null ? "OK" : "NULL"} — starting services');

  final onboardingDone = prefs?.getBool('onboarding_completed') ?? false;

  // Init Firebase + Sentry, then start the real app.
  String? initError;
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

/// Init Firebase only (no SharedPreferences — called in background).
Future<void> _initFirebase() async {
  try {
    await Firebase.initializeApp().timeout(
      const Duration(seconds: 5),
      onTimeout: () {
        throw TimeoutException('Firebase timeout (5s)');
      },
    );
    FirebaseMessaging.onBackgroundMessage(firebaseMessagingBackgroundHandler);
    debugPrint('[init] Firebase OK (background)');
  } catch (e) {
    debugPrint('[init] Firebase background init failed: $e — non-critical');
  }
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

/// Start the real app. Safe to call multiple times (idempotent).
void _startApp(
  SharedPreferences? prefs,
  bool onboardingDone, {
  String? initError,
}) {
  if (_appStarted) return; // Already started (by timer or by _initServices)
  _appStarted = true;
  debugPrint('[start] App starting — onboardingDone=$onboardingDone, prefs=${prefs != null}, error=$initError');

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
