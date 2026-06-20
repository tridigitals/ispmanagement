import 'package:dio/dio.dart';

import '../auth/auth_token_storage.dart';
import 'package:flutter/foundation.dart';

import 'api_endpoints.dart';

/// Holds runtime configuration for the API client.
@immutable
class ApiConfig {
  const ApiConfig({
    required this.baseUrl,
    this.timeout = const Duration(seconds: 15),
    this.enableLogging = kDebugMode,
    this.maxRetries = 1,
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
      defaultValue: 'http://103.190.112.214:3000',
    );
    return const ApiConfig(baseUrl: baseUrl);
  }
}

/// Build a configured [Dio] instance with auth interceptor + retry.
///
/// [onReLogin] — optional async callback that attempts to re-authenticate on
/// 401. Should return the new token string on success, or null on failure.
/// When provided and biometric is enabled, the interceptor will attempt
/// auto re-login instead of immediately clearing the session.

/// ⚠️ GLOBAL FALLBACK TOKEN — set synchronously by apply() right after login.
/// This is the BELT-AND-SUSPENDERS guarantee. AuthInterceptor checks this
/// STATIC variable FIRST, before any storage read, cache, or Dio options.
/// On Android 12/13, FlutterSecureStorage.read() can race with the in-memory
/// cache even though _cachedToken was set synchronously — this static ensures
/// the token is available for the FIRST API call after login (me() in loading
/// screen), which is the one that matters. It's cleared on logout/clear().
String? _globalLatestToken;

/// Set the global fallback token (called from apply() after login success).
void setGlobalAuthToken(String? token) {
  _globalLatestToken = token;
}

/// Clear the global fallback token (called from logout / tokenStorage.clear()).
void clearGlobalAuthToken() {
  _globalLatestToken = null;
}

Dio buildDio({
  required ApiConfig config,
  required AuthTokenStorage tokenStorage,
  Future<String?> Function()? onReLogin,
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

  dio.interceptors.add(AuthInterceptor(
    tokenStorage: tokenStorage,
    onReLogin: onReLogin,
  ));
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

/// Attaches Bearer token; on 401, clears stored token and propagates the error.
///
/// If [onReLogin] is provided (e.g. from the mobile app which can attempt
/// auto re-login with stored credentials + biometric), the interceptor will
/// attempt to re-authenticate on the first 401 before clearing the session.
class AuthInterceptor extends Interceptor {
  AuthInterceptor({required this.tokenStorage, this.onReLogin});

  final AuthTokenStorage tokenStorage;
  final Future<String?> Function()? onReLogin;
  bool _isRefreshing = false;

  @override
  Future<void> onRequest(
    RequestOptions options,
    RequestInterceptorHandler handler,
  ) async {
    // 1. Check global fallback token FIRST (set synchronously by apply()).
    //    This is the fastest path and bypasses ALL storage reads — critical
    //    for the first API call after login (me() in loading screen) on
    //    Android 12/13 where FlutterSecureStorage can race the in-memory cache.
    if (_globalLatestToken != null && _globalLatestToken!.isNotEmpty) {
      options.headers['Authorization'] = 'Bearer $_globalLatestToken';
      handler.next(options);
      return;
    }

    // 2. Check if auth header was already pre-set on Dio options headers.
    //    This catches cases where apply() set dio.options.headers directly.
    final existing = options.headers['Authorization'] as String?;
    if (existing != null && existing.isNotEmpty) {
      handler.next(options);
      return;
    }

    // 3. Fall back to storage read (with in-memory cache inside readToken()).
    final token = await tokenStorage.readToken();
    if (token != null) {
      options.headers['Authorization'] = 'Bearer $token';
    }
    handler.next(options);
  }

  @override
  Future<void> onError(DioException err, ErrorInterceptorHandler handler) async {
    if (err.response?.statusCode == 401) {
      // Avoid retry loops — if this was already a retried request, skip.
      if (err.requestOptions.extra['is_401_retry'] == true) {
        await tokenStorage.clear();
        return handler.next(err);
      }

      if (onReLogin != null) {
        if (_isRefreshing) {
          // Another request is already refreshing — wait briefly and retry.
          await Future<void>.delayed(const Duration(seconds: 2));
          final newToken = await tokenStorage.readToken();
          if (newToken != null && newToken.isNotEmpty) {
            err.requestOptions.headers['Authorization'] = 'Bearer $newToken';
            err.requestOptions.extra['is_401_retry'] = true;
            try {
              final dio = Dio(BaseOptions(baseUrl: err.requestOptions.baseUrl));
              final response = await dio.fetch(err.requestOptions);
              return handler.resolve(response);
            } on DioException catch (e) {
              return handler.next(e);
            }
          }
        }

        _isRefreshing = true;
        try {
          final newToken = await onReLogin!();
          if (newToken != null && newToken.isNotEmpty) {
            // Re-login succeeded — retry the original request.
            err.requestOptions.headers['Authorization'] = 'Bearer $newToken';
            err.requestOptions.extra['is_401_retry'] = true;
            final dio = Dio(BaseOptions(baseUrl: err.requestOptions.baseUrl));
            final response = await dio.fetch(err.requestOptions);
            return handler.resolve(response);
          }
        } catch (_) {
          // Re-login failed — fall through to clear token.
        } finally {
          _isRefreshing = false;
        }
      }

      // Re-login not available or failed — clear session.
      await tokenStorage.clear();
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
