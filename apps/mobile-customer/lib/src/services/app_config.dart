import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../router/app_router.dart';

/// Configuration for the mobile app. Edit `.env` or pass `--dart-define`s.
class AppConfig {
  static const apiBaseUrl = String.fromEnvironment(
    'API_BASE_URL',
    defaultValue: 'https://api-isp-management.tridigitals.com',
  );

  static const wsBaseUrl = String.fromEnvironment(
    'WS_BASE_URL',
    defaultValue: 'wss://api-isp-management.tridigitals.com',
  );

  static const appName = 'ISP Customer';
  static const appVersion = '0.1.0';
}

/// Provides the [ApiConfig] for the API client.
final apiConfigProvider = Provider<ApiConfig>((ref) {
  return ApiConfig(
    baseUrl: AppConfig.apiBaseUrl,
    enableLogging: true,
  );
});

/// Secure token storage — single instance for the lifetime of the app.
final tokenStorageProvider = Provider<AuthTokenStorage>((ref) {
  return AuthTokenStorage();
});

/// The configured [Dio] HTTP client with auth + retry interceptors.
final dioProvider = Provider<Dio>((ref) {
  final config = ref.watch(apiConfigProvider);
  final storage = ref.watch(tokenStorageProvider);
  return buildDio(config: config, tokenStorage: storage);
});
