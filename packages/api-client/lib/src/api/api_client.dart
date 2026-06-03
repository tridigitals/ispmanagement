import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import 'api_endpoints.dart';

/// Holds runtime configuration for the API client.
@immutable
class ApiConfig {
  const ApiConfig({
    required this.baseUrl,
    this.timeout = const Duration(seconds: 30),
    this.enableLogging = kDebugMode,
    this.maxRetries = 3,
  });

  /// Base URL of the API (e.g. `https://api-isp-management.tridigitals.com`).
  final String baseUrl;

  /// HTTP request timeout.
  final Duration timeout;

  /// Enable Dio logging in debug.
  final bool enableLogging;

  /// Number of automatic retries on transient failure.
  final int maxRetries;

  /// Default config — uses the production endpoint, but is overridable.
  static ApiConfig fromEnv() {
    const baseUrl = String.fromEnvironment(
      'API_BASE_URL',
      defaultValue: 'https://api-isp-management.tridigitals.com',
    );
    return const ApiConfig(baseUrl: baseUrl);
  }
}

/// Secure storage for auth tokens.
class AuthTokenStorage {
  AuthTokenStorage({FlutterSecureStorage? storage})
      : _storage = storage ??
            const FlutterSecureStorage(
              aOptions: AndroidOptions(encryptedSharedPreferences: true),
              iOptions: IOSOptions(
                accessibility: KeychainAccessibility.first_unlock,
              ),
            );

  static const _kTokenKey = 'auth_token';
  static const _kRefreshKey = 'refresh_token';
  static const _kUserIdKey = 'user_id';
  static const _kTenantIdKey = 'tenant_id';

  final FlutterSecureStorage _storage;

  Future<void> save({
    required String token,
    String? refreshToken,
    String? userId,
    String? tenantId,
  }) async {
    await _storage.write(key: _kTokenKey, value: token);
    if (refreshToken != null) {
      await _storage.write(key: _kRefreshKey, value: refreshToken);
    }
    if (userId != null) {
      await _storage.write(key: _kUserIdKey, value: userId);
    }
    if (tenantId != null) {
      await _storage.write(key: _kTenantIdKey, value: tenantId);
    }
  }

  Future<String?> readToken() => _storage.read(key: _kTokenKey);
  Future<String?> readRefresh() => _storage.read(key: _kRefreshKey);
  Future<String?> readUserId() => _storage.read(key: _kUserIdKey);
  Future<String?> readTenantId() => _storage.read(key: _kTenantIdKey);

  Future<void> clear() async {
    await _storage.delete(key: _kTokenKey);
    await _storage.delete(key: _kRefreshKey);
    await _storage.delete(key: _kUserIdKey);
    await _storage.delete(key: _kTenantIdKey);
  }
}

/// Build a configured [Dio] instance with auth interceptor + retry.
Dio buildDio({
  required ApiConfig config,
  required AuthTokenStorage tokenStorage,
  TokenRefreshCallback? onTokenRefresh,
}) {
  final dio = Dio(
    BaseOptions(
      baseUrl: config.baseUrl,
      connectTimeout: config.timeout,
      sendTimeout: config.timeout,
      receiveTimeout: config.timeout,
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
        'X-Client-Platform': 'mobile-flutter',
      },
      responseType: ResponseType.json,
    ),
  );

  dio.interceptors.add(AuthInterceptor(tokenStorage: tokenStorage, onTokenRefresh: onTokenRefresh));
  dio.interceptors.add(RetryInterceptor(maxRetries: config.maxRetries));
  if (config.enableLogging) {
    dio.interceptors.add(LogInterceptor(
      request: true,
      requestBody: true,
      responseBody: true,
      error: true,
      logPrint: (o) => debugPrint('[API] $o'),
    ));
  }
  return dio;
}

/// Callback fired when a 401 is received and a fresh token is required.
typedef TokenRefreshCallback = Future<String?> Function();

/// Attaches Bearer token; on 401, attempts refresh and retries the request.
class AuthInterceptor extends Interceptor {
  AuthInterceptor({required this.tokenStorage, this.onTokenRefresh});

  final AuthTokenStorage tokenStorage;
  final TokenRefreshCallback? onTokenRefresh;

  @override
  Future<void> onRequest(
    RequestOptions options,
    RequestInterceptorHandler handler,
  ) async {
    final token = await tokenStorage.readToken();
    if (token != null) {
      options.headers['Authorization'] = 'Bearer $token';
    }
    handler.next(options);
  }

  @override
  Future<void> onError(DioException err, ErrorInterceptorHandler handler) async {
    final response = err.response;
    final isAuthFailure = response?.statusCode == 401;
    final isRefreshPath = err.requestOptions.path.contains('/auth/refresh');
    if (isAuthFailure && !isRefreshPath && onTokenRefresh != null) {
      final newToken = await onTokenRefresh!();
      if (newToken != null) {
        final retryOptions = err.requestOptions;
        retryOptions.headers['Authorization'] = 'Bearer $newToken';
        try {
          final dio = Dio(BaseOptions(
            baseUrl: retryOptions.baseUrl,
            headers: retryOptions.headers,
          ));
          final response = await dio.fetch(retryOptions);
          return handler.resolve(response);
        } catch (_) {
          // fall through
        }
      }
    }
    handler.next(err);
  }
}

/// Retries on 5xx and network errors with exponential backoff.
class RetryInterceptor extends Interceptor {
  RetryInterceptor({this.maxRetries = 3});
  final int maxRetries;

  @override
  Future<void> onError(DioException err, ErrorInterceptorHandler handler) async {
    final attempt = (err.requestOptions.extra['retry_attempt'] as int?) ?? 0;
    if (attempt >= maxRetries) return handler.next(err);

    final shouldRetry = _isRetriable(err);
    if (!shouldRetry) return handler.next(err);

    err.requestOptions.extra['retry_attempt'] = attempt + 1;
    final delay = Duration(milliseconds: 200 * (1 << attempt));
    await Future<void>.delayed(delay);

    final dio = Dio(BaseOptions(baseUrl: err.requestOptions.baseUrl));
    try {
      final response = await dio.fetch(err.requestOptions);
      handler.resolve(response);
    } on DioException catch (e) {
      handler.next(e);
    }
  }

  bool _isRetriable(DioException e) {
    if (e.type == DioExceptionType.connectionTimeout ||
        e.type == DioExceptionType.receiveTimeout ||
        e.type == DioExceptionType.sendTimeout ||
        e.type == DioExceptionType.connectionError) {
      return true;
    }
    final code = e.response?.statusCode ?? 0;
    return code >= 500 && code < 600;
  }
}

/// Thrown when an API call fails with a known error payload.
class ApiException implements Exception {
  ApiException({
    required this.message,
    this.statusCode,
    this.code,
    this.details,
  });

  factory ApiException.fromDio(DioException e) {
    final data = e.response?.data;
    if (data is Map<String, dynamic>) {
      return ApiException(
        message: (data['message'] as String?) ?? (data['error'] as String?) ?? e.message ?? 'Unknown error',
        statusCode: e.response?.statusCode,
        code: data['code'] as String?,
        details: data,
      );
    }
    return ApiException(
      message: e.message ?? 'Network error',
      statusCode: e.response?.statusCode,
    );
  }

  final String message;
  final int? statusCode;
  final String? code;
  final Map<String, dynamic>? details;

  bool get isUnauthorized => statusCode == 401;
  bool get isForbidden => statusCode == 403;
  bool get isNotFound => statusCode == 404;
  bool get isRateLimited => statusCode == 429;

  @override
  String toString() => 'ApiException($statusCode): $message';
}
