import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app_config.dart';
import 'service_providers.dart';
import 'package:result_dart/result_dart.dart';

/// Holds the customer's notifications and provides mark-read actions.
class NotificationsNotifier
    extends AsyncNotifier<List<NotificationModel>> {
  @override
  Future<List<NotificationModel>> build() async {
    final svc = ref.watch(notificationServiceProvider);
    final res = await svc.list(page: 1, perPage: 50);
    return switch (res) {
      Success(:final data) => data.data,
      Failure(:final exception) => throw exception.message,
    };
  }

  Future<void> markRead(String id) async {
    final current = state.valueOrNull;
    if (current == null) return;
    // Optimistic update.
    state = AsyncData([
      for (final n in current)
        if (n.id == id) n.copyWith(readAt: DateTime.now()) else n,
    ]);
    await ref.read(notificationServiceProvider).markRead(id);
  }

  Future<void> markAllRead() async {
    final current = state.valueOrNull;
    if (current == null) return;
    final now = DateTime.now();
    state = AsyncData([for (final n in current) n.copyWith(readAt: now)]);
    await ref.read(notificationServiceProvider).markAllRead();
  }
}

final notificationsProvider =
    AsyncNotifierProvider<NotificationsNotifier, List<NotificationModel>>(
  NotificationsNotifier.new,
);

/// Lightweight badge count for the home tab.
final unreadNotificationsCountProvider = FutureProvider<int>((ref) async {
  // Re-fetch when the inbox changes.
  ref.watch(notificationsProvider);
  final svc = ref.watch(notificationServiceProvider);
  final res = await svc.unreadCount();
  return switch (res) {
    Success(:final data) => data,
    Failure() => 0,
  };
});
