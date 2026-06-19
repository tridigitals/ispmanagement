import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'service_providers.dart';

/// Public settings from /api/settings/public — includes payment gateway flags
/// and active bank accounts for manual transfer.
final publicSettingsProvider = FutureProvider<PublicSettingsModel>((ref) async {
  final svc = ref.watch(authServiceProvider);
  final res = await svc.settingsPublic();
  switch (res) {
    case Success(:final data):
      return data;
    case Failure(:final exception):
      throw Exception(exception.message);
  }
});
