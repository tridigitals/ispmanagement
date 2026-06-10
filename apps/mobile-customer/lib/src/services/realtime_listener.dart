import 'dart:async';

import 'package:api_client/api_client.dart' hide Success, Failure;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'notifications_providers.dart';
import 'service_providers.dart';

/// Listens to the realtime WebSocket stream and shows in-app notifications
/// when new events arrive (foreground only — no push).
///
/// Place this widget above the Navigator in the widget tree.
/// It auto-connects when authenticated and disconnects on dispose.
class RealtimeNotificationListener extends ConsumerStatefulWidget {
  const RealtimeNotificationListener({super.key, required this.child});
  final Widget child;

  @override
  ConsumerState<RealtimeNotificationListener> createState() =>
      _RealtimeNotificationListenerState();
}

class _RealtimeNotificationListenerState
    extends ConsumerState<RealtimeNotificationListener> {
  StreamSubscription<Map<String, dynamic>>? _sub;

  @override
  void initState() {
    super.initState();
    // Defer to after first build so providers are available.
    WidgetsBinding.instance.addPostFrameCallback((_) => _connect());
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  void _connect() {
    final client = ref.read(realtimeClientProvider);
    _sub?.cancel();
    _sub = client.stream.listen(_onEvent);
  }

  void _onEvent(Map<String, dynamic> event) {
    final type = event['type'] as String?;

    switch (type) {
      case 'notification':
        _handleNotification(event['data'] as Map<String, dynamic>?);
        break;
      case 'ticket_reply':
        _handleTicketReply(event['data'] as Map<String, dynamic>?);
        break;
      case 'payment_status':
        _handlePaymentStatus(event['data'] as Map<String, dynamic>?);
        break;
      default:
        // Unknown event — ignore silently
        break;
    }
  }

  /// New notification from backend → inject into state + show snackbar.
  void _handleNotification(Map<String, dynamic>? data) {
    if (data == null) return;

    try {
      final notification = NotificationModel.fromJson(data);

      // Inject into the notification list provider (prepend).
      ref.read(notificationsProvider.notifier).injectRealtime(notification);

      // Show non-intrusive snackbar.
      _showNotificationSnackbar(
        icon: _iconForCategory(notification.category),
        title: notification.title,
        body: notification.body,
      );
    } catch (_) {
      // Malformed payload — ignore
    }
  }

  void _handleTicketReply(Map<String, dynamic>? data) {
    if (data == null) return;
    final subject = data['subject'] as String? ?? 'Tiket';
    _showNotificationSnackbar(
      icon: Icons.support_agent,
      title: 'Balasan baru',
      body: subject,
    );
    // Trigger notification list refresh.
    ref.invalidate(notificationsProvider);
  }

  void _handlePaymentStatus(Map<String, dynamic>? data) {
    if (data == null) return;
    final status = data['status'] as String? ?? '';
    final amount = data['amount']?.toString() ?? '';
    final icon = status == 'success'
        ? Icons.check_circle
        : status == 'pending'
            ? Icons.pending
            : Icons.error;

    _showNotificationSnackbar(
      icon: icon,
      title: status == 'success' ? 'Pembayaran berhasil' : 'Status pembayaran',
      body: amount.isNotEmpty ? 'Rp $amount' : '',
    );
    ref.invalidate(notificationsProvider);
  }

  void _showNotificationSnackbar({
    required IconData icon,
    required String title,
    String? body,
  }) {
    if (!mounted) return;
    final messenger = ScaffoldMessenger.of(context);
    messenger
      ..clearSnackBars()
      ..showSnackBar(
        SnackBar(
          behavior: SnackBarBehavior.floating,
          margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          shape:
              RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
          duration: const Duration(seconds: 4),
          content: Row(
            children: [
              Icon(icon, color: Colors.white, size: 20),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      style: const TextStyle(
                        fontWeight: FontWeight.w600,
                        fontSize: 13,
                        color: Colors.white,
                      ),
                    ),
                    if (body != null && body.isNotEmpty)
                      Text(
                        body,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                            fontSize: 12, color: Colors.white70),
                      ),
                  ],
                ),
              ),
            ],
          ),
          action: SnackBarAction(
            label: 'Lihat',
            textColor: Colors.white,
            onPressed: () {
              // Navigate to notification inbox or trigger detail.
              // GoRouter is available above this widget.
              messenger.clearSnackBars();
            },
          ),
        ),
      );
  }

  IconData _iconForCategory(NotificationCategory category) {
    switch (category) {
      case NotificationCategory.invoice:
        return Icons.receipt_long;
      case NotificationCategory.ticket:
        return Icons.support_agent;
      case NotificationCategory.outage:
        return Icons.warning_amber;
      case NotificationCategory.payment:
        return Icons.payment;
      case NotificationCategory.subscription:
        return Icons.wifi;
      case NotificationCategory.promo:
        return Icons.local_offer;
      case NotificationCategory.system:
        return Icons.info_outline;
    }
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
