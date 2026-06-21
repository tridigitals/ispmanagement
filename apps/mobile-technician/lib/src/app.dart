import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import 'l10n/app_localizations.dart';
import 'router/app_router.dart';
import 'services/app_config.dart';
import 'services/auth_providers.dart';
import 'services/fcm_service.dart';
import 'services/gps_service.dart';
import 'services/realtime_listener.dart';
import 'services/settings_providers.dart';
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
    // GPS tracker: start 2s after launch (only effective if user is logged in).
    _scheduleGpsStart();
    // FCM safety net: also call init after bootstrap completes, regardless
    // of whether the auth state changed. The `ref.listen` below covers the
    // state-change case, but a cold start with a still-valid session may
    // not transition through the listener reliably (and the FCM token can
    // be invalidated by Firebase at any time). init() is idempotent and
    // uses force=false on this path, so it's a no-op if already done.
    _scheduleFcmBootstrap();
  }

  /// Run FCM init 1.5s after app start. By that point bootstrap() has
  /// either restored the session or fallen back to the login screen. We
  /// unconditionally try to init — FCM service itself is idempotent and
  /// will no-op if the user isn't authenticated yet.
  void _scheduleFcmBootstrap() {
    Future.delayed(const Duration(milliseconds: 1500), () {
      // ignore: discarded_futures
      ref.read(fcmServiceProvider).init();
    });
    // Second safety net: if the first attempt silently failed (e.g.,
    // Firebase was still warming up), try again 8s after start. The
    // FcmService guards against double-init via the _inFlight flag.
    Future.delayed(const Duration(seconds: 8), () {
      // ignore: discarded_futures
      ref.read(fcmServiceProvider).init();
    });
  }

  /// GPS — start 2s after app launch, but ONLY if a user is authenticated.
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
    // Keep router in sync with auth state.
    ref.listen<AuthState>(authControllerProvider, (prev, next) {
      if (prev?.isAuthenticated != next.isAuthenticated) {
        _router.refresh();
        // Init FCM + GPS after login. Use force=true so a logout→login cycle
        // re-registers the device token.
        if (next.isAuthenticated) {
          ref.read(fcmServiceProvider).init(force: true);
          ref.read(gpsTrackingServiceProvider).start();
        }
      }
    });

    return MaterialApp.router(
      title: _config.appTitle,
      theme: AppTheme.light(),
      darkTheme: AppTheme.dark(),
      themeMode: ref.watch(themeModeProvider),
      locale: ref.watch(localeProvider),
      debugShowCheckedModeBanner: false,
      routerConfig: _router,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) {
        // Lock orientation to portrait (mobile-only app).
        final mq = MediaQuery.of(context);
        Widget w = MediaQuery(
          data: mq.copyWith(
            textScaler:
                mq.textScaler.clamp(minScaleFactor: 0.9, maxScaleFactor: 1.3),
          ),
          child: IspToastOverlay(
            child: RealtimeNotificationListener(
              child: child ?? const SizedBox.shrink(),
            ),
          ),
        );

        // Log init errors to console only — don't show a banner that
        // interferes with the UI.  Firebase failures are non-critical.
        if (widget.initError != null) {
          debugPrint('[app] Init warning: ${widget.initError}');
        }

        return w;
      },
    );
  }
}
