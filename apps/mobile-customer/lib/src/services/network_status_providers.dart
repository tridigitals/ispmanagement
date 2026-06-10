import 'package:api_client/api_client.dart' hide Success, Failure;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'feature_providers.dart';

/// Network operational status for the customer's area.
final networkStatusProvider = FutureProvider<NetworkStatusModel>((ref) async {
  final svc = ref.watch(networkStatusServiceProvider);
  final res = await svc.getStatus();
  return res.fold(
    (value) => value,
    (error) => throw Exception(error.message),
  );
});
