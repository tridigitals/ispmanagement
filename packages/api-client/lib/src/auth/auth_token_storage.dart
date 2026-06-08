import 'package:flutter/foundation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Secure storage for auth tokens. Backed by:
/// - Android: EncryptedSharedPreferences (AES-256)
/// - iOS: Keychain (first unlock)
/// - Web: localStorage with encryption
/// - Desktop: OS keychain (libsecret on Linux, Credential Manager on Win)
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

  Future<void> save({
    required String token,
    String? refreshToken,
    String? userId,
    String? tenantId,
    DateTime? expiresAt,
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
    if (expiresAt != null) {
      await _storage.write(
        key: _kTokenExpiryKey,
        value: expiresAt.toIso8601String(),
      );
    }
  }

  Future<String?> readToken() => _storage.read(key: _kTokenKey);
  Future<String?> readRefresh() => _storage.read(key: _kRefreshKey);
  Future<String?> readUserId() => _storage.read(key: _kUserIdKey);
  Future<String?> readTenantId() => _storage.read(key: _kTenantIdKey);
  Future<DateTime?> readExpiry() async {
    final raw = await _storage.read(key: _kTokenExpiryKey);
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
    await _storage.write(key: _kEmailKey, value: email);
    await _storage.write(key: _kPasswordKey, value: password);
  }

  /// Read stored email for auto re-login.
  Future<String?> readEmail() => _storage.read(key: _kEmailKey);

  /// Read stored password for auto re-login.
  Future<String?> readPassword() => _storage.read(key: _kPasswordKey);

  Future<void> clear() async {
    await _storage.delete(key: _kTokenKey);
    await _storage.delete(key: _kRefreshKey);
    await _storage.delete(key: _kUserIdKey);
    await _storage.delete(key: _kTenantIdKey);
    await _storage.delete(key: _kTokenExpiryKey);
    await _storage.delete(key: _kEmailKey);
    await _storage.delete(key: _kPasswordKey);
  }

  @visibleForTesting
  FlutterSecureStorage get storageForTest => _storage;
}
