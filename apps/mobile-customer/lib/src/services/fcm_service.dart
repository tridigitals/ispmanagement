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
  final title = data['title'] as String? ?? 'ISP Customer';
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

  /// Initialize FCM — call after every successful login.
  ///
  /// The Riverpod provider keeps the same instance alive for the app's
  /// lifetime, so on logout→login we still need a way to retry token
  /// registration. We expose [force] for that case and re-run register
  /// every time we land on an authenticated state.
  Future<void> init({bool force = false}) async {
    if (_initialized && !force) return;
    _initialized = true;
    debugPrint('[FCM] init() running (force=$force)');

    try {
      // Local notification init.
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

      // Request permission (Android 13+).
      final settings = await FirebaseMessaging.instance.requestPermission(
        alert: true,
        badge: true,
        sound: true,
        provisional: false,
      );
      debugPrint('[FCM] Permission: ${settings.authorizationStatus}');

      // Listen foreground messages.
      FirebaseMessaging.onMessage.listen(_onForegroundMessage);

      // Listen tap when app in background.
      FirebaseMessaging.onMessageOpenedApp.listen(_onMessageTapped);

      // Cold start — check if opened from notification.
      final initial = await FirebaseMessaging.instance.getInitialMessage();
      if (initial != null) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          _onMessageTapped(initial);
        });
      }

      // Register token.
      await _registerToken();

      // Listen for token refresh.
      FirebaseMessaging.instance.onTokenRefresh.listen((_) => _registerToken());
    } catch (e, st) {
      debugPrint('[FCM] Init error: $e\n$st');
    }
  }

  /// Register FCM token with backend.
  Future<void> _registerToken() async {
    try {
      final token = await FirebaseMessaging.instance.getToken();
      if (token == null) {
        debugPrint('[FCM] Token is null!');
        return;
      }
      debugPrint('[FCM] Token: ${token.substring(0, 20)}...');

      final dio = _ref.read(dioProvider);
      await dio.post(
        '/api/notifications/devices',
        data: {
          'fcm_token': token,
          'platform': 'android',
        },
      );
      debugPrint('[FCM] Token registered OK');
    } catch (e) {
      debugPrint('[FCM] Token registration failed: $e');
    }
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
  void _navigateToAction(String? actionUrl) {
    try {
      final navKey = _ref.read(navigatorKeyProvider);
      final context = navKey.currentContext;
      if (context == null) return;

      String route;
      if (actionUrl == null || actionUrl.isEmpty) {
        route = '/notifications';
      } else if (actionUrl.startsWith('/support/')) {
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

      GoRouter.of(context).go(route);
    } catch (e) {
      debugPrint('[FCM] Navigation error: $e');
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
