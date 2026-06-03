import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'service_providers.dart';

/// Customer's own subscriptions, paginated.
final mySubscriptionsProvider =
    FutureProvider<PaginatedResponse<SubscriptionModel>>((ref) async {
  final svc = ref.watch(subscriptionServiceProvider);
  final res = await svc.list(page: 1, perPage: 20);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw _ApiError(exception),
  };
});

/// Customer's own invoices.
final myInvoicesProvider =
    FutureProvider<PaginatedResponse<InvoiceModel>>((ref) async {
  final svc = ref.watch(invoiceServiceProvider);
  final res = await svc.list(page: 1, perPage: 20);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw _ApiError(exception),
  };
});

/// Customer's own support tickets.
final myTicketsProvider =
    FutureProvider<PaginatedResponse<TicketModel>>((ref) async {
  final svc = ref.watch(ticketServiceProvider);
  final res = await svc.list(page: 1, perPage: 20);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw _ApiError(exception),
  };
});

class _ApiError implements Exception {
  _ApiError(this.api);
  final ApiException api;
  @override
  String toString() => api.message;
}
