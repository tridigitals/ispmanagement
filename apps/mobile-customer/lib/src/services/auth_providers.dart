import 'package:api_client/api_client.dart';
import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:local_auth/local_auth.dart';

import 'app_config.dart';
import 'missing_providers.dart';
import 'notifications_providers.dart';
import 'public_settings_providers.dart';
import 'service_providers.dart';

/// Converts any exception to a user-friendly ApiException.
/// Parses DioException to extract server-side error messages.
ApiException _toApiException(Object e) {
  if (e is DioException) return ApiException.fromDio(e);
  return ApiException(message: e.toString());
}

/// Two-factor enrollment data (stub).
class TwoFactorEnrollment {
  const TwoFactorEnrollment({
    required this.enrollmentId,
    required this.secret,
    required this.otpAuthUri,
    this.periodSeconds = 30,
    this.backupCodes = const [],
  });
  final String enrollmentId;
  final String secret;
  final String otpAuthUri;
  final int periodSeconds;
  final List<String> backupCodes;
}

class AuthState {
  const AuthState({this.user, this.isLoading = false});
  final UserModel? user;
  final bool isLoading;

  bool get isAuthenticated => user != null;
  AuthState copyWith(
      {UserModel? user, bool? isLoading, bool clearUser = false}) {
    return AuthState(
      user: clearUser ? null : (user ?? this.user),
      isLoading: isLoading ?? this.isLoading,
    );
  }
}

/// Holds the current authenticated user and login state.
class AuthController extends Notifier<AuthState> {
  @override
  AuthState build() => const AuthState();

  Future<ServiceResult<AuthResponse>> login({
    required String email,
    required String password,
  }) async {
    state = state.copyWith(isLoading: true);
    try {
      final res = await ref.read(authServiceProvider).login(
            email: email,
            password: password,
          );
      // Save credentials for auto re-login on 401 (used with biometric).
      // This is fire-and-forget — it MUST NOT block the login flow. The
      // credentials are only used for biometric re-login on 401, which is
      // a non-critical path. If storage is wedged, we don't want login to hang.
      switch (res) {
        case Success(:final data):
          // ignore: unawaited_futures, discarded_futures
          Future(() async {
            try {
              final storage = ref.read(tokenStorageProvider);
              await storage
                  .saveCredentials(email: email, password: password)
                  .timeout(const Duration(seconds: 5));
            } catch (e) {
              debugPrint('[auth] saveCredentials failed: $e');
            }
          });
          if (!data.requires2fa && !data.requires2faSetup) {
            // apply() enforces customer-only role. If the login response
            // came back with a staff/admin user, apply() returns Failure
            // and we must propagate it so the UI shows the rejection
            // message instead of silently dropping the user into the app.
            final applied = await apply(data);
            if (applied is Failure<bool>) {
              // Surface the role-rejection message in place of the success
              // response so the login screen snackbar shows the right text
              // and the router does NOT navigate to the home shell.
              return Failure(applied.exception);
            }
          }
        case Failure():
          break;
      }
      return res;
    } catch (e) {
      debugPrint('[auth] login error: $e');
      return Failure(_toApiException(e));
    } finally {
      state = state.copyWith(isLoading: false);
    }
  }

  Future<ServiceResult<AuthResponse>> verify2fa({
    required String tempToken,
    required String code,
    String method = 'totp',
  }) async {
    state = state.copyWith(isLoading: true);
    final res = await ref.read(authServiceProvider).verify2fa(
          tempToken: tempToken,
          code: code,
          method: method,
        );
    state = state.copyWith(isLoading: false);
    return res;
  }

  Future<ServiceResult<bool>> forgotPassword({
    required String email,
    String? reason,
  }) async {
    return ref.read(authServiceProvider).forgotPassword(email);
  }

  Future<ServiceResult<TwoFactorEnrollment>> start2faEnroll() async {
    try {
      final dio = ref.read(dioProvider);
      final res = await dio.post<Map<String, dynamic>>('/api/auth/2fa/enable');
      final data = res.data ?? const {};
      final secret = (data['secret'] as String?) ?? '';
      // Backend returns {secret, qr} — construct otpAuthUri from secret
      final otpAuthUri =
          'otpauth://totp/ISPManagement:$secret?secret=$secret&issuer=ISPManagement&algorithm=SHA1&digits=6&period=30';
      return Success(
        TwoFactorEnrollment(
          enrollmentId: secret, // use secret as identifier
          secret: secret,
          otpAuthUri: otpAuthUri,
          periodSeconds: 30,
          backupCodes: const [],
        ),
      );
    } on Exception catch (e) {
      return Failure(_toApiException(e));
    }
  }

  Future<ServiceResult<bool>> confirm2faEnroll({
    required String enrollmentId,
    required String code,
  }) async {
    try {
      final dio = ref.read(dioProvider);
      await dio.post(
        '/api/auth/2fa/verify-setup',
        data: {
          'secret': enrollmentId, // enrollmentId is actually the secret
          'code': code,
        },
      );
      // Refresh user state
      final me = await ref.read(authServiceProvider).me();
      if (me is Success<UserModel>) {
        state = AuthState(user: me.data);
      }
      return const Success(true);
    } on Exception catch (e) {
      return Failure(_toApiException(e));
    }
  }

  // ── Temp-token 2FA enrollment (forced setup at login) ──

  Future<ServiceResult<TwoFactorEnrollment>> start2faEnrollTemp(
    String tempToken,
  ) async {
    try {
      final dio = ref.read(dioProvider);
      final res = await dio.post<Map<String, dynamic>>(
        '/api/auth/2fa/temp/enable',
        data: {'tempToken': tempToken},
      );
      final data = res.data ?? const {};
      final secret = (data['secret'] as String?) ?? '';
      final otpAuthUri =
          'otpauth://totp/ISPManagement:$secret?secret=$secret&issuer=ISPManagement&algorithm=SHA1&digits=6&period=30';
      return Success(
        TwoFactorEnrollment(
          enrollmentId: secret,
          secret: secret,
          otpAuthUri: otpAuthUri,
          periodSeconds: 30,
          backupCodes: const [],
        ),
      );
    } on Exception catch (e) {
      return Failure(_toApiException(e));
    }
  }

  Future<ServiceResult<AuthResponse>> confirm2faEnrollTemp({
    required String tempToken,
    required String secret,
    required String code,
  }) async {
    try {
      final dio = ref.read(dioProvider);
      final res = await dio.post<Map<String, dynamic>>(
        '/api/auth/2fa/temp/verify-setup',
        data: {'tempToken': tempToken, 'secret': secret, 'code': code},
      );
      final authResponse = AuthResponse.fromJson(res.data ?? const {});
      // Apply session
      await apply(authResponse);
      return Success(authResponse);
    } on Exception catch (e) {
      return Failure(_toApiException(e));
    }
  }

  Future<ServiceResult<AuthResponse>> confirmEmail2faEnrollTemp({
    required String tempToken,
    required String code,
  }) async {
    try {
      final dio = ref.read(dioProvider);
      final res = await dio.post<Map<String, dynamic>>(
        '/api/auth/2fa/temp/email/enable-verify',
        data: {'tempToken': tempToken, 'code': code},
      );
      final authResponse = AuthResponse.fromJson(res.data ?? const {});
      await apply(authResponse);
      return Success(authResponse);
    } on Exception catch (e) {
      return Failure(_toApiException(e));
    }
  }

  Future<ServiceResult<bool>> changePassword({
    required String current,
    required String next,
  }) async {
    try {
      final dio = ref.read(dioProvider);
      await dio.post(
        '/api/auth/change-password',
        data: {
          'current_password': current,
          'new_password': next,
        },
      );
      return const Success(true);
    } on Exception catch (e) {
      return Failure(_toApiException(e));
    }
  }

  Future<ServiceResult<bool>> updateProfile({
    String? name,
    String? phone,
    String? email,
  }) async {
    try {
      final dio = ref.read(dioProvider);
      await dio.put(
        '/api/auth/me',
        data: {
          if (name != null) 'name': name,
          if (phone != null) 'phone': phone,
          if (email != null) 'email': email,
        },
      );
      // Refresh user state
      final me = await ref.read(authServiceProvider).me();
      if (me is Success<UserModel>) {
        state = AuthState(user: me.data);
      }
      return const Success(true);
    } on Exception catch (e) {
      return Failure(_toApiException(e));
    }
  }

  /// Upload avatar (base64 image content). Returns the new avatar URL.
  Future<ServiceResult<String>> uploadAvatar(String base64Content) async {
    try {
      final dio = ref.read(dioProvider);
      final res = await dio.post<Map<String, dynamic>>(
        '/api/auth/avatar',
        data: {'content': base64Content},
      );
      final data = res.data;
      if (data == null) {
        return Failure(ApiException(message: 'Empty response from server'));
      }
      final avatarUrl = data['avatar_url'] as String?;
      if (avatarUrl == null || avatarUrl.isEmpty) {
        return Failure(ApiException(message: 'No avatar URL returned'));
      }
      // Refresh user state with new avatar
      final me = await ref.read(authServiceProvider).me();
      if (me is Success<UserModel>) {
        state = AuthState(user: me.data);
      }
      return Success(avatarUrl);
    } on Exception catch (e) {
      return Failure(_toApiException(e));
    }
  }

  Future<ServiceResult<bool>> disable2fa({String? code}) async {
    try {
      final dio = ref.read(dioProvider);
      final response = await dio.post(
        '/api/auth/2fa/disable',
        data: code != null ? {'code': code} : {},
      );

      // Check if OTP verification is required
      if (response.data is Map && response.data['requires_verification'] == true) {
        // Return special result indicating OTP is needed
        return Failure(ApiException(
          message: 'requires_verification',
          statusCode: 200,
        ));
      }

      // Refresh user state
      final me = await ref.read(authServiceProvider).me();
      if (me is Success<UserModel>) {
        state = AuthState(user: me.data);
      }
      return const Success(true);
    } on Exception catch (e) {
      return Failure(_toApiException(e));
    }
  }

  Future<ServiceResult<bool>> tryBiometricUnlock() async {
    try {
      final auth = LocalAuthentication();
      final canCheck = await auth.canCheckBiometrics;
      if (!canCheck) {
        // Device doesn't support biometric — fall back to session restore
        await bootstrap();
        return state.isAuthenticated
            ? const Success(true)
            : Failure(ApiException(message: 'Biometric not available'));
      }
      final ok = await auth.authenticate(
        localizedReason: 'Gunakan fingerprint untuk membuka aplikasi',
        options: const AuthenticationOptions(
          stickyAuth: true,
          biometricOnly: true,
        ),
      );
      if (!ok) {
        return Failure(ApiException(message: 'Biometric auth cancelled'));
      }
      await bootstrap();
      if (state.isAuthenticated) {
        return const Success(true);
      }
      return Failure(ApiException(message: 'No session found'));
    } on Exception catch (e) {
      return Failure(_toApiException(e));
    }
  }

  /// Persist the auth response (token + user) into local state.
  ///
  /// Token storage uses an in-memory cache (set in save() before any
  /// native I/O), so this method does NOT block on the native storage
  /// even if Android Keystore is wedged. The persistSession().timeout(5s)
  /// remains as a safety net but should rarely fire.
  ///
  /// Enforces customer-only role: the customer APK is for end users
  /// subscribing to ISP services. Staff/admin/super_admin accounts
  /// must use the admin web app instead. Non-customer logins are
  /// rejected with [Failure] — session is rolled back (token cleared
  /// from storage + in-memory cache, user state reset) so a stale
  /// session cannot leak across a role mismatch.
  Future<ServiceResult<bool>> apply(AuthResponse auth) async {
    // Role gate: only `customer` role may use this app. Backend already
    // gates login by tenant + credentials; this is a client-side guard
    // against accidentally letting a staff account slip through.
    if (!auth.user.isCustomer) {
      debugPrint(
        '[auth] rejected non-customer login: '
        'role=${auth.user.role} user=${auth.user.email}',
      );
      // Best-effort cleanup: clear any token we just persisted. If storage
      // is wedged, the in-memory cache below also gets wiped so a stray
      // request can't slip through.
      try {
        await ref.read(authServiceProvider).logout()
            .timeout(const Duration(seconds: 3));
      } catch (e) {
        debugPrint('[auth] clearSession during role-reject failed: $e');
      }
      state = const AuthState();
      return Failure(
        ApiException(
          message:
              'Akun ini bukan akun pelanggan. APK ini hanya untuk pengguna '
              'layanan internet. Silakan login di aplikasi admin.',
        ),
      );
    }
    try {
      await ref.read(authServiceProvider).persistSession(auth)
          .timeout(const Duration(seconds: 5));
    } catch (e) {
      debugPrint('[auth] persistSession timed out (relying on cache): $e');
      // Still set user state — in-memory cache ensures auth flow works
      // for this session even if storage write hung.
    }
    state = AuthState(user: auth.user);
    return const Success(true);
  }

  /// Hydrate from secure storage on app start.
  /// On failure, do NOT delete token — might be transient (network/server).
  /// Token stays so user can retry or login manually (which will overwrite it).
  ///
  /// Enforces customer-only role on session restore too — if a staff/admin
  /// token leaked into this APK (e.g. from a prior install of the wrong app,
  /// or a hand-edited token), we wipe the session and return false so the
  /// router keeps the user on the login screen.
  Future<bool> bootstrap() async {
    final auth = ref.read(authServiceProvider);
    if (!await auth.hasSession()) return false;
    final me = await auth.me();
    switch (me) {
      case Success(:final data):
        if (!data.isCustomer) {
          debugPrint(
            '[auth] bootstrap rejected non-customer: '
            'role=${data.role} user=${data.email}',
          );
          try {
            await auth.logout().timeout(const Duration(seconds: 3));
          } catch (e) {
            debugPrint('[auth] logout during bootstrap role-reject failed: $e');
          }
          state = const AuthState();
          return false;
        }
        state = AuthState(user: data);
        return true;
      case Failure():
        // Keep token. User can retry or login with email/password.
        return false;
    }
  }

  Future<void> logout({bool force = false}) async {
    // If biometric is enabled and not forcing, just lock (clear in-memory user).
    // Token stays in storage so fingerprint can restore session.
    final bioEnabled = ref.read(biometricEnabledProvider).valueOrNull ?? false;
    if (bioEnabled && !force) {
      state = const AuthState(user: null);
      return;
    }

    await ref.read(authServiceProvider).logout();

    // Invalidate ALL user-specific cached providers so a different user logging
    // in gets fresh data — not stale cache from the previous user.
    ref.invalidate(notificationsProvider);
    ref.invalidate(mySubscriptionsProvider);
    ref.invalidate(myInvoicesProvider);
    ref.invalidate(myTicketsProvider);
    ref.invalidate(unreadNotificationsCountProvider);
    ref.invalidate(activeAnnouncementsProvider);
    ref.invalidate(publicSettingsProvider);

    state = const AuthState(user: null);
  }
}

final authControllerProvider =
    NotifierProvider<AuthController, AuthState>(AuthController.new);

/// Current user — convenience derived from auth controller.
final currentUserProvider = Provider<UserModel?>((ref) {
  return ref.watch(authControllerProvider).user;
});

/// Used by `GoRouter.refreshListenable` to rebuild routes on auth change.
final authStateProvider = Provider<GoRouterRefresh>((ref) {
  return GoRouterRefresh(ref);
});

/// Bridges Riverpod auth state to a `Listenable` for GoRouter.
class GoRouterRefresh extends ChangeNotifier {
  GoRouterRefresh(Ref ref) {
    ref.listen<AuthState>(authControllerProvider, (_, __) => notifyListeners());
  }
}
