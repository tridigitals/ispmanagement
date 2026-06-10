import 'dart:async';

import 'package:api_client/api_client.dart' hide Success, Failure;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'feature_providers.dart';

/// Holds the customer's notifications and provides mark-read actions.
/// Auto-refreshes every 60 seconds to pick up new notifications.
class NotificationsNotifier extends AsyncNotifier<List<NotificationModel>> {
  Timer? _pollTimer;
  int _currentPage = 1;
  bool _hasMore = true;

  /// Whether more pages are available for loading.
  bool get hasMore => _hasMore;

  @override
  Future<List<NotificationModel>> build() async {
    // Start periodic polling (15s — short until FCM background push is ready).
    _pollTimer?.cancel();
    _pollTimer = Timer.periodic(
      const Duration(seconds: 15),
      (_) => _refresh(),
    );
    ref.onDispose(() => _pollTimer?.cancel());

    _currentPage = 1;
    _hasMore = true;
    return _fetch();
  }

  Future<List<NotificationModel>> _fetch() async {
    final svc = ref.watch(notificationServiceProvider);
    final res = await svc.list(page: 1, perPage: 50);
    return res.fold(
      (paginated) {
        _hasMore = paginated.hasMore;
        _currentPage = 1;
        return paginated.data;
      },
      (error) => throw Exception(error.message),
    );
  }

  /// Silent refresh — doesn't show loading state.
  Future<void> _refresh() async {
    try {
      final data = await _fetch();
      state = AsyncData(data);
    } catch (_) {
      // Silent fail on background poll — keep existing state
    }
  }

  /// Load next page and append to existing state.
  Future<bool> loadMore() async {
    if (!_hasMore) return false;
    final svc = ref.read(notificationServiceProvider);
    final nextPage = _currentPage + 1;
    final res = await svc.list(page: nextPage, perPage: 50);
    return res.fold(
      (paginated) {
        final current = state.valueOrNull ?? [];
        _currentPage = nextPage;
        _hasMore = paginated.hasMore;
        state = AsyncData([...current, ...paginated.data]);
        return true;
      },
      (error) => false,
    );
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

  /// Inject a realtime notification (WebSocket) into the state.
  void injectRealtime(NotificationModel notification) {
    final current = state.valueOrNull ?? [];
    if (current.any((n) => n.id == notification.id)) return; // dedupe
    state = AsyncData([notification, ...current]);
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
  return res.fold(
    (value) => value,
    (error) => 0,
  );
});
