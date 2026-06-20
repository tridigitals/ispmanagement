import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import 'app_config.dart';

/// Secure storage for persisting biometric flag and other sensitive data.
final secureStorageProvider = Provider<FlutterSecureStorage>((ref) {
  return const FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );
});

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

final storageServiceProvider = Provider<StorageService>((ref) {
  return StorageService(dio: ref.watch(dioProvider));
});

final notificationServiceProvider = Provider<NotificationService>((ref) {
  return NotificationService(dio: ref.watch(dioProvider));
});

final locationServiceProvider = Provider<LocationService>((ref) {
  return LocationService(dio: ref.watch(dioProvider));
});

final realtimeClientProvider = Provider<RealtimeClient>((ref) {
  final cfg = ref.watch(appConfigProvider);
  return RealtimeClient(
    baseUrl: cfg.wsBaseUrl,
    tokenStorage: ref.watch(tokenStorageProvider),
  );
});
