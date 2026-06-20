import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';

import 'l10n/app_localizations.dart';
import 'router/app_router.dart';
import 'services/app_config.dart';
import 'services/auth_providers.dart';
import 'services/fcm_service.dart';
import 'services/gps_service.dart';
import 'services/missing_providers.dart';
import 'theme/app_theme.dart';

class IspTechnicianApp extends ConsumerStatefulWidget {
  const IspTechnicianApp({super.key, this.initError});
  /// Optional error message from main() init phase (Firebase, SharedPreferences, etc.)
  final String? initError;
  @override
  ConsumerState<IspTechnicianApp> createState() => _State();
}

class _State extends ConsumerState<IspTechnicianApp> {
  late final AppConfig _config;
  late final GoRouter _router;
  late final ProviderContainer _container;
  bool _initialized = false;

  @override
  void initState() {
    super.initState();
    _config = AppConfig.fromEnv();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_initialized) return;
    _initialized = true;
    _container = ProviderScope.containerOf(context, listen: false);
    _router = buildAppRouter(
      ref: ref,
      container: _container,
      navigatorKey: ref.read(navigatorKeyProvider),
    );
    ref.read(apiClientProvider); // eagerly init
    // Restore auth session from secure storage on app start.
    Future.microtask(
        () => ref.read(authControllerProvider.notifier).bootstrap());
    // FCM safety net: schedule init 1.5s after start.
    _scheduleFcmBootstrap();
    // GPS tracker: start 2s after launch (only effective if user is logged in).
    _scheduleGpsStart();
  }

  void _scheduleFcmBootstrap() {
    Future.delayed(const Duration(milliseconds: 1500), () async {
      try {
        if (Firebase.apps.isNotEmpty) {
          await ref.read(fcmServiceProvider).init(force: false);
        }
      } catch (e) {
        debugPrint('[fcm] bootstrap init failed: $e');
      }
    });
  }

  /// GPS — start 2s after app launch, but ONLY if a user is authenticated.
  /// Login screen shouldn't trigger GPS (no token yet). The service itself
  /// will silently no-op if permission is denied.
  void _scheduleGpsStart() {
    Future.delayed(const Duration(seconds: 2), () {
      final auth = ref.read(authControllerProvider);
      if (auth.isAuthenticated) {
        ref.read(gpsTrackingServiceProvider).start();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp.router(
      title: 'ISP Technician',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.light(),
      darkTheme: AppTheme.dark(),
      routerConfig: _router,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
    );
  }
}