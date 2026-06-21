import 'dart:async';

import 'package:api_client/api_client.dart' hide Success, Failure;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'notifications_providers.dart';
import 'service_providers.dart';

/// Listens to the realtime WebSocket stream and updates providers
/// when new events arrive (foreground only — no push).
///
/// Place this widget above the Navigator in the widget tree.
/// It auto-connects when authenticated and disconnects on dispose.
/// Reconnects automatically when app resumes from background.
class RealtimeNotificationListener extends ConsumerStatefulWidget {
  const RealtimeNotificationListener({super.key, required this.child});
  final Widget child;

  @override
  ConsumerState<RealtimeNotificationListener> createState() =>
      _RealtimeNotificationListenerState();
}

class _RealtimeNotificationListenerState
    extends ConsumerState<RealtimeNotificationListener>
    with WidgetsBindingObserver {
  StreamSubscription<Map<String, dynamic>>? _sub;
  RealtimeClient? _channel;
  bool _connected = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    // Defer to after first build so providers are available.
    WidgetsBinding.instance.addPostFrameCallback((_) => _connect());
  }

  void _connect() {
    final client = ref.read(realtimeClientProvider);
    _channel = client;
    _sub?.cancel();
    _sub = client.stream.listen(_onEvent);
    client.connect();
    _connected = true;
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _sub?.cancel();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      // App came back from background — force reconnect the WebSocket.
      // Android may have silently killed the TCP connection while backgrounded.
      debugPrint('[RealtimeListener] App resumed, forcing WS reconnect');
      _channel?.forceReconnect();
    }
  }

  void _onEvent(Map<String, dynamic> event) {
    final type = event['type'] as String?;

    switch (type) {
      case 'notification':
      case 'notification_received':
        _handleNotification(event);
        break;
      case 'ticket_reply':
      case 'support_ticket_message_created':
        _handleTicketReply(event);
        break;
      case 'payment_status':
        _handlePaymentStatus(event);
        break;
      default:
        // Unknown event — ignore silently
        break;
    }
  }

  /// New notification from backend → inject into provider (bell count updates).
  void _handleNotification(Map<String, dynamic>? data) {
    if (data == null) return;
    try {
      final notification = NotificationModel.fromJson(data);
      ref.read(notificationsProvider.notifier).injectRealtime(notification);
    } catch (_) {
      // Malformed payload — ignore
    }
  }

  void _handleTicketReply(Map<String, dynamic>? data) {
    if (data == null) return;
    // Refresh notification list — bell count updates.
    ref.invalidate(notificationsProvider);
  }

  void _handlePaymentStatus(Map<String, dynamic>? data) {
    if (data == null) return;
    ref.invalidate(notificationsProvider);
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
