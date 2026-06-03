import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app_config.dart';

/// All shared service providers wired into Riverpod.
final authServiceProvider = Provider<AuthService>((ref) {
  return AuthService(
    dio: ref.watch(dioProvider),
    tokenStorage: ref.watch(tokenStorageProvider),
  );
});

final subscriptionServiceProvider = Provider<SubscriptionService>((ref) {
  return SubscriptionService(dio: ref.watch(dioProvider));
});

final invoiceServiceProvider = Provider<InvoiceService>((ref) {
  return InvoiceService(dio: ref.watch(dioProvider));
});

final ticketServiceProvider = Provider<TicketService>((ref) {
  return TicketService(dio: ref.watch(dioProvider));
});

final realtimeClientProvider = Provider<RealtimeClient>((ref) {
  return RealtimeClient(
    baseUrl: AppConfig.wsBaseUrl,
    tokenStorage: ref.watch(tokenStorageProvider),
  );
});
