import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import 'l10n/app_localizations.dart';
import 'router/app_router.dart';
import 'services/app_config.dart';
import 'services/auth_providers.dart';
import 'services/fcm_service.dart';
import 'services/realtime_listener.dart';
import 'services/settings_providers.dart';
import 'theme/app_theme.dart';

class IspCustomerApp extends ConsumerStatefulWidget {
  const IspCustomerApp({super.key});
  @override
  ConsumerState<IspCustomerApp> createState() => _State();
}

class _State extends ConsumerState<IspCustomerApp> {
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
  }

  @override
  Widget build(BuildContext context) {
    // Keep router in sync with auth state.
    ref.listen<AuthState>(authControllerProvider, (prev, next) {
      if (prev?.isAuthenticated != next.isAuthenticated) {
        _router.refresh();
        // Init FCM after login. Use force=true so a logout→login cycle
        // re-registers the device token (the Riverpod provider keeps the
        // same FcmService instance alive across auth state changes).
        if (next.isAuthenticated) {
          ref.read(fcmServiceProvider).init(force: true);
        }
      }
    });

    return MaterialApp.router(
      title: _config.appTitle,
      theme: AppTheme.light(),
      darkTheme: AppTheme.dark(),
      themeMode: ref.watch(themeModeProvider),
      debugShowCheckedModeBanner: false,
      routerConfig: _router,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) {
        // Lock orientation to portrait (mobile-only app).
        final mq = MediaQuery.of(context);
        return MediaQuery(
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
      },
    );
  }
}
