import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import '../api/api_client.dart';

/// Secure storage for auth tokens. Backed by:
/// - Android: EncryptedSharedPreferences (AES-256)
/// - iOS: Keychain (first unlock)
/// - Web: localStorage with encryption
/// - Desktop: OS keychain (libsecret on Linux, Credential Manager on Win)
///
/// ⚠️ All read operations are wrapped with a 5s timeout + try-catch.
/// FlutterSecureStorage.read() is known to hang on Android 12/13 (some
/// devices) — without this guard, EVERY API call (including the login
/// request itself via AuthInterceptor.onRequest) would block forever,
/// showing the login button in a perpetual loading state.
class AuthTokenStorage {
  AuthTokenStorage({FlutterSecureStorage? storage})
      : _storage = storage ??
            const FlutterSecureStorage(
              // encryptedSharedPreferences: false uses the regular Android Keystore
              // with AES encryption per-value (not the AndroidX EncryptedSharedPreferences
              // wrapper). This avoids the "Keystore operation failed" error on some
              // Android devices where EncryptedSharedPreferences initialization hangs
              // or throws GeneralSecurityException.
              aOptions: AndroidOptions(encryptedSharedPreferences: false),
              iOptions: IOSOptions(
                accessibility: KeychainAccessibility.first_unlock,
              ),
            );

  static const _kTokenKey = 'auth_token';
  static const _kRefreshKey = 'refresh_token';
  static const _kUserIdKey = 'user_id';
  static const _kTenantIdKey = 'tenant_id';
  static const _kTokenExpiryKey = 'token_expiry';

  final FlutterSecureStorage _storage;

  // ── In-memory cache ────────────────────────────────────────────────
  // FlutterSecureStorage is backed by Android Keystore (EncryptedSharedPreferences).
  // On some Android 12/13 devices, every read/write can hang for 5-30+ seconds
  // while the keystore locks the prefs file. Without a cache, every API call
  // (which goes through AuthInterceptor.onRequest → readToken) would block.
  //
  // Strategy: set the cache synchronously on save(), and read from cache first
  // on readToken(). The auth flow works for the current session even if the
  // native storage is wedged. On app restart, the cache is empty and the
  // user will need to re-login if storage truly failed to persist.
  String? _cachedToken;

  /// Read a key with a 5s timeout + try-catch. Returns null on any failure.
  /// This is the ONLY entry point to `_storage.read` — never call directly.
  Future<String?> _safeRead(String key) async {
    try {
      return await _storage.read(key: key).timeout(
            const Duration(seconds: 5),
            onTimeout: () {
              debugPrint('[auth] _storage.read($key) timed out (5s)');
              return null;
            },
          );
    } catch (e) {
      debugPrint('[auth] _storage.read($key) failed: $e');
      return null;
    }
  }

  /// Write a key with a 5s timeout + try-catch. Returns true on success.
  Future<bool> _safeWrite(String key, String? value) async {
    try {
      await _storage
          .write(key: key, value: value)
          .timeout(const Duration(seconds: 5));
      return true;
    } catch (e) {
      debugPrint('[auth] _storage.write($key) failed: $e');
      return false;
    }
  }

  Future<void> save({
    required String token,
    String? refreshToken,
    String? userId,
    String? tenantId,
    DateTime? expiresAt,
  }) async {
    // 1. Set in-memory cache SYNCHRONOUSLY — this is the source of truth for
    //    the active session. readToken() will return this without touching
    //    the (potentially wedged) native storage. AuthInterceptor.onRequest
    //    → readToken() is called on every API call, so this is the hot path.
    _cachedToken = token;

    // 2. AWAIT storage writes. This is critical — if we fire-and-forget,
    //    the auth flow continues before the write completes. The next API call
    //    (home screen fetch on first login) calls readToken() → storage empty
    //    → no token → 401. We use _safeWrite which has 5s timeout + try-catch
    //    so a wedged Android Keystore doesn't block login.
    try {
      await _safeWrite(_kTokenKey, token);
      if (refreshToken != null) {
        await _safeWrite(_kRefreshKey, refreshToken);
      }
      if (userId != null) {
        await _safeWrite(_kUserIdKey, userId);
      }
      if (tenantId != null) {
        await _safeWrite(_kTenantIdKey, tenantId);
      }
      if (expiresAt != null) {
        await _safeWrite(
          _kTokenExpiryKey,
          expiresAt.toIso8601String(),
        );
      }
    } catch (e) {
      debugPrint('[auth] persistSession failed (cache still valid): $e');
    }
  }

  /// Read token — cache first, storage as fallback.
  /// On app restart, cache is empty so we fall through to storage.
  Future<String?> readToken() async {
    if (_cachedToken != null && _cachedToken!.isNotEmpty) {
      return _cachedToken;
    }
    final t = await _safeRead(_kTokenKey);
    if (t != null && t.isNotEmpty) {
      _cachedToken = t; // populate cache from storage on cold start
    }
    return t;
  }
  Future<String?> readRefresh() => _safeRead(_kRefreshKey);
  Future<String?> readUserId() => _safeRead(_kUserIdKey);
  Future<String?> readTenantId() => _safeRead(_kTenantIdKey);
  Future<DateTime?> readExpiry() async {
    final raw = await _safeRead(_kTokenExpiryKey);
    if (raw == null) return null;
    return DateTime.tryParse(raw);
  }

  Future<bool> isExpired() async {
    final expiry = await readExpiry();
    if (expiry == null) return false; // no expiry set
    return DateTime.now().isAfter(expiry);
  }

  // ── Credential storage for auto re-login on 401 ──
  static const _kIdentifierKey = 'stored_identifier';
  static const _kPasswordKey = 'stored_password';

  /// Save login credentials (identifier = email OR phone) for auto re-login on 401.
  Future<void> saveCredentials({
    required String identifier,
    required String password,
  }) async {
    await _safeWrite(_kIdentifierKey, identifier);
    await _safeWrite(_kPasswordKey, password);
  }

  /// Read stored identifier (email OR phone) for auto re-login.
  Future<String?> readIdentifier() => _safeRead(_kIdentifierKey);

  /// Read stored password for auto re-login.
  Future<String?> readPassword() => _safeRead(_kPasswordKey);

  Future<void> clear() async {
    // Clear global fallback token FIRST — auth interceptor reads this
    // synchronously before any storage path.
    clearGlobalAuthToken();
    // Clear in-memory cache FIRST — auth flow is immediately logged out.
    _cachedToken = null;
    // Use individual safe deletes — clear() is best-effort during logout
    // and a hang here would freeze the logout flow.
    for (final key in [
      _kTokenKey,
      _kRefreshKey,
      _kUserIdKey,
      _kTenantIdKey,
      _kTokenExpiryKey,
      _kIdentifierKey,
      _kPasswordKey,
    ]) {
      try {
        await _storage.delete(key: key).timeout(const Duration(seconds: 5));
      } catch (e) {
        debugPrint('[auth] _storage.delete($key) failed: $e');
      }
    }
  }

  @visibleForTesting
  FlutterSecureStorage get storageForTest => _storage;
}
