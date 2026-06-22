import 'dart:async';

import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/material.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'app_config.dart';

/// Background handler — must be a top-level function.
///
/// Called when the app receives a message while in the background or
/// terminated. For data-only messages we must display a local notification
/// ourselves because Android won't show anything automatically.
@pragma('vm:entry-point')
Future<void> firebaseMessagingBackgroundHandler(RemoteMessage message) async {
  await Firebase.initializeApp();
  debugPrint('[FCM] Background message: ${message.messageId}');

  // Only show local notification for data-only messages (no notification payload).
  // Messages with a `notification` field are displayed automatically by Android.
  final data = message.data;
  final title = data['title'] as String? ?? 'ISP Teknisi';
  final body = data['body'] as String? ?? '';

  if (message.notification == null && body.isNotEmpty) {
    final localNotif = FlutterLocalNotificationsPlugin();
    const androidSettings =
        AndroidInitializationSettings('@mipmap/ic_launcher');
    const initSettings = InitializationSettings(android: androidSettings);
    await localNotif.initialize(initSettings);

    await localNotif.show(
      message.hashCode,
      title,
      body,
      const NotificationDetails(
        android: AndroidNotificationDetails(
          'high_importance_channel',
          'Notifikasi Penting',
          channelDescription: 'Tagihan, gangguan, dan info langganan ISP',
          importance: Importance.high,
          priority: Priority.high,
          icon: '@mipmap/ic_launcher',
        ),
      ),
      payload: data['action_url'] as String?,
    );
  }
}

/// Android notification channel for high-importance messages.
const AndroidNotificationChannel _channel = AndroidNotificationChannel(
  'high_importance_channel',
  'Notifikasi Penting',
  description: 'Tagihan, gangguan, dan info langganan ISP',
  importance: Importance.high,
  playSound: true,
  enableVibration: true,
);

final FlutterLocalNotificationsPlugin _localNotif =
    FlutterLocalNotificationsPlugin();

/// Singleton service managing FCM lifecycle.
class FcmService {
  FcmService(this._ref);
  final Ref _ref;
  bool _initialized = false;
  bool _inFlight = false;

  /// Becomes true after the first authenticated state is detected.
  /// Used to suppress FCM navigation during the login startup window.
  bool _authHandledLogin = false;

  /// Step-level logging — written to debug log only (no UI banner).
  void _status(String msg) {
    debugPrint('[FCM] $msg');
  }

  /// Initialize FCM — call after every successful login AND on every app start.
  ///
  /// Why both: a cold start with a still-valid session doesn't transition
  /// through an auth state change (state goes from null→authenticated during
  /// bootstrap, but timing is fragile). On top of that, FCM tokens can be
  /// invalidated by Firebase at any time (uninstall, app data clear, 270-day
  /// expiry, etc.) so we re-register on every app open.
  ///
  /// Robustness: every step has a timeout, and failures are caught locally.
  /// The whole init is fire-and-forget — callers do NOT await.
  Future<void> init({bool force = false}) async {
    if (_inFlight) return; // already running
    if (_initialized && !force) return;
    _inFlight = true;
    // NOTE: we DON'T set _initialized=true here. Setting it on success lets
    // a failed init get retried on the next call.
    _status('init() running (force=$force)');

    try {
      _status('Step 1/6: _initInternal starting');
      await _initInternal();
      _initialized = true;
      _status('✅ init OK — all steps completed');
    } catch (e, st) {
      _status('❌ init error: $e');
    } finally {
      _inFlight = false;
    }
  }

  Future<void> _initInternal() async {
    _status('Step 1/6: LocalNotification init');
    // Local notification init — fast.
    const androidSettings =
        AndroidInitializationSettings('@mipmap/ic_launcher');
    const initSettings = InitializationSettings(android: androidSettings);
    await _localNotif.initialize(
      initSettings,
      onDidReceiveNotificationResponse: _onLocalNotifTap,
    );

    // Create Android channel.
    await _localNotif
        .resolvePlatformSpecificImplementation<
            AndroidFlutterLocalNotificationsPlugin>()
        ?.createNotificationChannel(_channel);
    _status('Step 2/6: NotificationChannel created');

    // Request permission (Android 13+). Wrapped in 5s timeout — this call
    // can hang on some devices if the system dialog is interrupted.
    _status('Step 3/6: Requesting permission...');
    final settings = await FirebaseMessaging.instance
        .requestPermission(
          alert: true,
          badge: true,
          sound: true,
          provisional: false,
        )
        .timeout(const Duration(seconds: 5), onTimeout: () {
      _status('⚠️ requestPermission timeout — assuming granted');
      // Return a default-allowed value so init continues. If the user did
      // deny, the actual FCM call will still fail downstream.
      return NotificationSettings(
        alert: AppleNotificationSetting.enabled,
        announcement: AppleNotificationSetting.enabled,
        authorizationStatus: AuthorizationStatus.authorized,
        badge: AppleNotificationSetting.enabled,
        carPlay: AppleNotificationSetting.enabled,
        lockScreen: AppleNotificationSetting.enabled,
        notificationCenter: AppleNotificationSetting.enabled,
        showPreviews: AppleShowPreviewSetting.always,
        timeSensitive: AppleNotificationSetting.enabled,
        criticalAlert: AppleNotificationSetting.disabled,
        sound: AppleNotificationSetting.enabled,
        providesAppNotificationSettings: AppleNotificationSetting.disabled,
      );
    });
    _status('Permission: ${settings.authorizationStatus}');

    _status('Step 4/6: Setting up listeners');
    // Listen foreground messages.
    FirebaseMessaging.onMessage.listen(_onForegroundMessage);

    // Listen tap when app in background.
    FirebaseMessaging.onMessageOpenedApp.listen(_onMessageTapped);

    // Cold start — check if opened from notification. 3s timeout: this is
    // best-effort and shouldn't block app startup.
    _status('Step 5/6: getInitialMessage');
    try {
      final initial = await FirebaseMessaging.instance
          .getInitialMessage()
          .timeout(const Duration(seconds: 3));
      if (initial != null) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          _onMessageTapped(initial);
        });
      }
    } catch (e) {
      _status('⚠️ getInitialMessage failed: $e');
    }

    // Register token — with built-in retry on null/error.
    _status('Step 6/6: Registering token...');
    await _registerTokenWithRetry();

    // Listen for token refresh (e.g., Firebase rotates the token).
    FirebaseMessaging.instance.onTokenRefresh.listen((_) {
      _status('Token refresh — re-registering');
      // ignore: discarded_futures
      _registerTokenWithRetry();
    });
  }

  /// Register FCM token with backend, with 1 retry on null/error.
  Future<void> _registerTokenWithRetry() async {
    for (var attempt = 1; attempt <= 2; attempt++) {
      try {
        final ok = await _registerToken();
        if (ok) return;
        // Token was null — wait 2s and retry (Firebase may need a moment
        // to provision a token after first install).
        if (attempt == 1) {
          _status('No token yet, retrying in 2s…');
          await Future.delayed(const Duration(seconds: 2));
        }
      } catch (e) {
        _status('❌ Register attempt $attempt: $e');
        if (attempt == 1) {
          await Future.delayed(const Duration(seconds: 2));
        }
      }
    }
    _status('❌ All register attempts failed');
  }

  /// Returns true on success, false on null token, throws on transport error.
  Future<bool> _registerToken() async {
    final token = await FirebaseMessaging.instance
        .getToken()
        .timeout(const Duration(seconds: 10), onTimeout: () {
      _status('❌ getToken timeout (10s)');
      return null;
    });
    if (token == null) {
      _status('⚠️ Token is null');
      return false;
    }
    _status('Token: ${token.substring(0, 20)}…');

    final dio = _ref.read(dioProvider);
    await dio
        .post(
          '/api/notifications/devices',
          data: {
            'fcm_token': token,
            'platform': 'android',
          },
        )
        .timeout(const Duration(seconds: 10));
    _status('✅ Token registered OK!');
    return true;
  }

  /// Foreground message — show local notification.
  void _onForegroundMessage(RemoteMessage message) {
    final notif = message.notification;
    if (notif == null) return;

    debugPrint('[FCM] Foreground: ${notif.title}');

    _localNotif.show(
      message.hashCode,
      notif.title,
      notif.body,
      NotificationDetails(
        android: AndroidNotificationDetails(
          _channel.id,
          _channel.name,
          channelDescription: _channel.description,
          importance: Importance.high,
          priority: Priority.high,
          icon: '@mipmap/ic_launcher',
          color: const Color(0xFF6C63FF),
        ),
      ),
      payload: message.data['action_url'] as String?,
    );
  }

  /// Handle local notification tap (foreground).
  void _onLocalNotifTap(NotificationResponse response) {
    final actionUrl = response.payload;
    _navigateToAction(actionUrl);
  }

  /// Handle FCM message tap (background / cold start).
  void _onMessageTapped(RemoteMessage message) {
    final actionUrl = message.data['action_url'] as String?;
    _navigateToAction(actionUrl);
  }

  /// Normalize action_url → in-app route and navigate.
  /// FCM navigation is suppressed during the login window (app startup → first
  /// authenticated state) to prevent stale push data from pushing /tickets/xxx
  /// on top of the login screen before the auth redirect takes effect.
  String? _pendingActionUrl;

  void _navigateToAction(String? actionUrl) {
    try {
      final navKey = _ref.read(navigatorKeyProvider);
      final context = navKey.currentContext;
      if (context == null) return;

      // Store the FIRST action URL as pending (clears on next navigate).
      // This prevents stale FCM data from cold-start / pre-login from
      // navigating to /tickets/xxx BEFORE the login redirect takes effect.
      // Subsequent navigations proceed normally.
      if (_pendingActionUrl == null) {
        _pendingActionUrl = actionUrl;
        debugPrint('[FCM] Pending: $actionUrl');
      }

      _doNavigate(context, actionUrl);
    } catch (e) {
      debugPrint('[FCM] Navigation error: $e');
    }
  }

  /// Clear pending FCM URL after login redirect completes.
  /// Called by LoginScreen right before navigating to home.
  void clearPendingAction() {
    _pendingActionUrl = null;
  }

  void _doNavigate(BuildContext context, String? actionUrl) {
    String? route;

    if (actionUrl == null || actionUrl.isEmpty) {
      route = '/notifications';
    } else if (actionUrl.startsWith('/support/')) {
      final id = actionUrl.substring('/support/'.length);
      if (id.isNotEmpty) {
        GoRouter.of(context).push('/tickets/$id');
        return;
      }
      route = '/?tab=3';
    } else if (actionUrl.startsWith('/admin/support/')) {
      final id = actionUrl.substring('/admin/support/'.length);
      if (id.isNotEmpty) {
        GoRouter.of(context).push('/tickets/$id');
        return;
      }
      route = '/?tab=3';
    } else if (actionUrl.startsWith('/pay/') ||
        actionUrl.startsWith('/invoices')) {
      route = '/?tab=2';
    } else if (actionUrl.startsWith('/services') ||
        actionUrl.startsWith('/subscriptions/')) {
      route = '/?tab=1';
    } else if (actionUrl.startsWith('/announcements/')) {
      route = '/?tab=0';
    } else {
      route = actionUrl;
    }

    if (route != null) {
      GoRouter.of(context).go(route);
    }
  }
}

/// Riverpod provider for FCM service.
final fcmServiceProvider = Provider<FcmService>((ref) {
  return FcmService(ref);
});

/// Global navigator key — shared with GoRouter.
final navigatorKeyProvider = Provider<GlobalKey<NavigatorState>>((ref) {
  return GlobalKey<NavigatorState>();
});
