import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'service_providers.dart';

/// Ticket stats filtered by role (admin: all in tenant, technician: assigned).
/// Auto-refreshes every 60 seconds to keep numbers fresh on the home screen.
final ticketStatsProvider = FutureProvider<TicketStats>((ref) async {
  // Re-fetch when invalidated.
  ref.watch(myTicketsProvider);
  final svc = ref.watch(ticketServiceProvider);
  final res = await svc.stats();
  return res.fold(
    (stats) => stats,
    (error) => throw Exception(error.message),
  );
});

/// Tickets assigned to (or created by) the current technician.
///
/// Pagination is stateful — `loadMore()` advances `_currentPage` and appends
/// new items to the existing list. Initial fetch uses page 1.
class MyTicketsNotifier extends AsyncNotifier<PaginatedResponse<TicketModel>> {
  Timer? _refreshTimer;

  @override
  Future<PaginatedResponse<TicketModel>> build() async {
    // Re-fetch every 60s so new assignments show up without manual refresh.
    _refreshTimer?.cancel();
    _refreshTimer = Timer.periodic(
      const Duration(seconds: 60),
      (_) => _silentRefresh(),
    );
    ref.onDispose(() => _refreshTimer?.cancel());

    final svc = ref.read(ticketServiceProvider);
    final res = await svc.list(page: 1, perPage: 50);
    return res.fold(
      (paginated) => paginated,
      (error) => throw Exception(error.message),
    );
  }

  Future<void> _silentRefresh() async {
    try {
      final svc = ref.read(ticketServiceProvider);
      final res = await svc.list(page: 1, perPage: 50);
      final paginated = res.fold((p) => p, (_) => null);
      if (paginated != null) {
        state = AsyncData(paginated);
      }
    } catch (_) {
      // Silent fail on background poll — keep existing state.
    }
  }

  /// Pull-to-refresh: re-fetch from page 1 and replace state.
  Future<void> refresh() async {
    state = const AsyncLoading();
    ref.invalidateSelf();
    await future;
  }
}

final myTicketsProvider =
    AsyncNotifierProvider<MyTicketsNotifier, PaginatedResponse<TicketModel>>(
  MyTicketsNotifier.new,
);

/// Convenience: today's tickets (status open or in_progress, any date).
final todaysTicketsProvider = Provider<List<TicketModel>>((ref) {
  final state = ref.watch(myTicketsProvider);
  return state.maybeWhen(
    data: (paginated) {
      final now = DateTime.now();
      return paginated.data
          .where((t) =>
              t.status == TicketStatus.open ||
              t.status == TicketStatus.inProgress ||
              t.status == TicketStatus.waitingStaff)
          .toList();
    },
    orElse: () => const [],
  );
});

/// Convenience: tickets currently in progress (for the action bar).
final inProgressTicketsProvider = Provider<List<TicketModel>>((ref) {
  final state = ref.watch(myTicketsProvider);
  return state.maybeWhen(
    data: (paginated) =>
        paginated.data.where((t) => t.status == TicketStatus.inProgress).toList(),
    orElse: () => const [],
  );
});

/// Fetch a single ticket by id.
final ticketByIdProvider =
    FutureProvider.family<TicketModel, String>((ref, id) async {
  final svc = ref.watch(ticketServiceProvider);
  final res = await svc.getById(id);
  return res.fold(
    (ticket) => ticket,
    (error) => throw Exception(error.message),
  );
});

/// Fetch messages for a ticket (auto-disposes when not watched).
final ticketMessagesProvider =
    FutureProvider.family<List<TicketMessageModel>, String>((ref, id) async {
  final svc = ref.watch(ticketServiceProvider);
  final res = await svc.listMessages(id);
  return res.fold(
    (messages) => messages,
    (error) => throw Exception(error.message),
  );
});