import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app_config.dart';
import 'service_providers.dart';
import 'package:result_dart/result_dart.dart';

/// Network operational status for the customer's area.
final networkStatusProvider =
    FutureProvider<NetworkStatusModel>((ref) async {
  final svc = ref.watch(networkStatusServiceProvider);
  final res = await svc.getStatus();
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw exception.message,
  };
});
