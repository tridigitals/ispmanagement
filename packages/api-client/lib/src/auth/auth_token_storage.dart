import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

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
              aOptions: AndroidOptions(encryptedSharedPreferences: true),
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
  }

  Future<String?> readToken() => _safeRead(_kTokenKey);
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
  static const _kEmailKey = 'stored_email';
  static const _kPasswordKey = 'stored_password';

  /// Save login credentials for auto re-login on 401.
  Future<void> saveCredentials({
    required String email,
    required String password,
  }) async {
    await _safeWrite(_kEmailKey, email);
    await _safeWrite(_kPasswordKey, password);
  }

  /// Read stored email for auto re-login.
  Future<String?> readEmail() => _safeRead(_kEmailKey);

  /// Read stored password for auto re-login.
  Future<String?> readPassword() => _safeRead(_kPasswordKey);

  Future<void> clear() async {
    // Use individual safe writes — clear() is best-effort during logout
    // and a hang here would freeze the logout flow.
    for (final key in [
      _kTokenKey,
      _kRefreshKey,
      _kUserIdKey,
      _kTenantIdKey,
      _kTokenExpiryKey,
      _kEmailKey,
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
