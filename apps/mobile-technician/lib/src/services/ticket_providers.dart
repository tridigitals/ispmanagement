import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'service_providers.dart';

/// All tickets assigned to the current technician.
final myTicketsProvider =
    FutureProvider<PaginatedResponse<TicketModel>>((ref) async {
  final svc = ref.watch(ticketServiceProvider);
  final result = await svc.list();
  return result.getOrThrow();
});

/// Ticket statistics (all, open, pending, closed).
final ticketStatsProvider = FutureProvider<TicketStats>((ref) async {
  final svc = ref.watch(ticketServiceProvider);
  final result = await svc.stats();
  return result.getOrThrow();
});

/// Today's active tickets (open + inProgress).
final todaysTicketsProvider = Provider<List<TicketModel>>((ref) {
  final tickets = ref.watch(myTicketsProvider);
  return tickets.whenOrNull(
        data: (paginated) => paginated.data
            .where((t) =>
                t.status.name == 'open' || t.status.name == 'inProgress')
            .toList(),
      ) ??
      [];
});
