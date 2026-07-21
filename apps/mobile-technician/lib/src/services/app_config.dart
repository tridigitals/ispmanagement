import 'package:api_client/api_client.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Configuration for the mobile app. Edit `.env` or pass `--dart-define`s.
class AppConfig {
  const AppConfig({
    required this.apiBaseUrl,
    required this.wsBaseUrl,
    required this.appName,
    required this.appVersion,
  });

  factory AppConfig.fromEnv() {
    return const AppConfig(
      apiBaseUrl: String.fromEnvironment(
        'API_BASE_URL',
        defaultValue: '',
      ),
      wsBaseUrl: String.fromEnvironment(
        'WS_BASE_URL',
        defaultValue: '',
      ),
      appName: 'ISP Teknisi',
      appVersion: '0.1.0+1',
    );
  }

  final String apiBaseUrl;
  final String wsBaseUrl;
  final String appName;
  final String appVersion;

  String get appTitle => appName;
}

/// Provides the [ApiConfig] for the API client.
final apiConfigProvider = Provider<ApiConfig>((ref) {
  final cfg = ref.watch(appConfigProvider);
  return ApiConfig(
    baseUrl: cfg.apiBaseUrl,
    enableLogging: true,
  );
});

/// App config — single source of truth (rebuildable on dart-define changes).
final appConfigProvider = Provider<AppConfig>((ref) => AppConfig.fromEnv());

/// Secure token storage — single instance for the lifetime of the app.
final tokenStorageProvider = Provider<AuthTokenStorage>((ref) {
  return AuthTokenStorage();
});

/// In-memory token holder — set synchronously from login response BEFORE
/// any navigation. This bypasses FlutterSecureStorage read race conditions
/// on Android 12/13. AuthInterceptor will use this first before falling
/// back to storage read.
///
/// Used by AuthLoadingScreen to verify token is ready before pre-fetching.
final inMemoryTokenProvider = StateProvider<String?>((ref) => null);

/// The configured [Dio] HTTP client with auth + retry interceptors.
final dioProvider = Provider<Dio>((ref) {
  final config = ref.watch(apiConfigProvider);
  final storage = ref.watch(tokenStorageProvider);
  return buildDio(
    config: config,
    tokenStorage: storage,
    onReLogin: () => _attemptReLogin(ref, storage),
  );
});

/// Attempt to re-authenticate using stored credentials.
/// Returns the new token on success, or null on failure.
///
/// Creates a fresh Dio (without the 401 interceptor) to avoid infinite loops.
Future<String?> _attemptReLogin(
  Ref ref,
  AuthTokenStorage storage,
) async {
  // Check if biometric is enabled by reading directly from secure storage.
  // This avoids circular imports with feature_providers.dart.
  const secure = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
  );
  String? bioVal;
  try {
    bioVal = await secure
        .read(key: 'biometric_enabled')
        .timeout(const Duration(seconds: 5), onTimeout: () => null);
  } catch (_) {
    // Storage hang on Android 12/13 — fall through, no re-login.
  }
  if (bioVal != 'true') return null;

  final identifier = await storage.readIdentifier();
  final password = await storage.readPassword();
  if (identifier == null || password == null || identifier.isEmpty || password.isEmpty) {
    return null;
  }

  // Use a plain Dio without interceptors to avoid recursion.
  final config = ref.read(apiConfigProvider);
  final plainDio = Dio(
    BaseOptions(
      baseUrl: config.baseUrl,
      connectTimeout: config.timeout,
      sendTimeout: config.timeout,
      receiveTimeout: config.timeout,
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json'
      },
    ),
  );
  final authSvc = AuthService(dio: plainDio, tokenStorage: storage);
  final result = await authSvc.login(identifier: identifier, password: password);
  switch (result) {
    case Success(:final data):
      await authSvc.persistSession(data);
      return data.token;
    case Failure():
      return null;
  }
}

/// The configured API client (alias for [dioProvider]).
final apiClientProvider = Provider<Dio>((ref) => ref.watch(dioProvider));
