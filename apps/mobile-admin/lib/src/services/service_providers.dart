import 'package:api_client/api_client.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app_config.dart';
import 'auth_providers.dart';

/// API configuration derived from app config.
final apiConfigProvider = Provider<ApiConfig>((ref) {
  final cfg = ref.watch(appConfigProvider);
  return ApiConfig(
    baseUrl: cfg.apiBaseUrl,
    enableLogging: true,
  );
});

/// Auth token storage — kept alive for the app lifetime so login
/// survives route changes.
final tokenStorageProvider = Provider<AuthTokenStorage>((ref) {
  return AuthTokenStorage();
});

/// Dio configured with auth interceptor that reads the JWT from
/// [tokenStorageProvider] on every request.
final dioProvider = Provider<Dio>((ref) {
  final config = ref.watch(apiConfigProvider);
  final storage = ref.watch(tokenStorageProvider);
  return buildDio(
    config: config,
    tokenStorage: storage,
    onReLogin: null, // admin doesn't auto re-login
  );
});

// ── Feature services ──────────────────────────────────────────

final ticketServiceProvider = Provider<TicketService>((ref) {
  return TicketService(dio: ref.watch(dioProvider));
});
