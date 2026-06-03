import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app_config.dart';
import 'service_providers.dart';

final notificationServiceProvider = Provider<NotificationService>((ref) {
  return NotificationService(dio: ref.watch(dioProvider));
});

final networkStatusServiceProvider = Provider<NetworkStatusService>((ref) {
  return NetworkStatusService(
    dio: ref.watch(dioProvider),
    tokenStorage: ref.watch(tokenStorageProvider),
  );
});

final paymentServiceProvider = Provider<PaymentService>((ref) {
  return PaymentService(dio: ref.watch(dioProvider));
});
