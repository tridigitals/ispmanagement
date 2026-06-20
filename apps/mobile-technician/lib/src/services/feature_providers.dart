import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app_config.dart';
import 'service_providers.dart';

final announcementServiceProvider = Provider<AnnouncementService>((ref) {
  return AnnouncementService(ref.watch(dioProvider));
});

final networkStatusServiceProvider = Provider<NetworkStatusService>((ref) {
  return NetworkStatusService(
    dio: ref.watch(dioProvider),
    tokenStorage: ref.watch(tokenStorageProvider),
  );
});