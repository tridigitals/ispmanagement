import 'package:api_client/api_client.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:local_auth/local_auth.dart';

import 'app_config.dart';
import 'missing_providers.dart';
import 'service_providers.dart';

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
    final res = await ref.read(authServiceProvider).login(
          email: email,
          password: password,
        );
    // Save credentials for auto re-login on 401 (used with biometric).
    switch (res) {
      case Success(:final data):
        final storage = ref.read(tokenStorageProvider);
        await storage.saveCredentials(email: email, password: password);
        if (!data.requires2fa && !data.requires2faSetup) {
          await apply(data);
        }
      case Failure():
        break;
    }
    state = state.copyWith(isLoading: false);
    return res;
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
      return Failure(ApiException(message: e.toString()));
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
      return Failure(ApiException(message: e.toString()));
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
      return Failure(ApiException(message: e.toString()));
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
      return Failure(ApiException(message: e.toString()));
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
      return Failure(ApiException(message: e.toString()));
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
      return Failure(ApiException(message: e.toString()));
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
      return Failure(ApiException(message: e.toString()));
    }
  }

  Future<ServiceResult<bool>> disable2fa() async {
    try {
      final dio = ref.read(dioProvider);
      await dio.post('/api/auth/2fa/disable');
      // Refresh user state
      final me = await ref.read(authServiceProvider).me();
      if (me is Success<UserModel>) {
        state = AuthState(user: me.data);
      }
      return const Success(true);
    } on Exception catch (e) {
      return Failure(ApiException(message: e.toString()));
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
      return Failure(ApiException(message: e.toString()));
    }
  }

  /// Persist the auth response (token + user) into local state.
  Future<void> apply(AuthResponse auth) async {
    await ref.read(authServiceProvider).persistSession(auth);
    state = AuthState(user: auth.user);
  }

  /// Hydrate from secure storage on app start.
  /// On failure, do NOT delete token — might be transient (network/server).
  /// Token stays so user can retry or login manually (which will overwrite it).
  Future<bool> bootstrap() async {
    final auth = ref.read(authServiceProvider);
    if (!await auth.hasSession()) return false;
    final me = await auth.me();
    switch (me) {
      case Success(:final data):
        state = AuthState(user: data);
        return true;
      case Failure():
        // Keep token. User can retry or login with email/password.
        return false;
    }
  }

  Future<void> logout({bool force = false}) async {
    // If biometric is enabled and not forcing, just lock (clear in-memory user)
    // Token stays in storage so fingerprint can restore session.
    final bioEnabled = ref.read(biometricEnabledProvider).valueOrNull ?? false;
    if (bioEnabled && !force) {
      state = const AuthState(user: null);
      return;
    }
    await ref.read(authServiceProvider).logout();
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
