import 'package:dio/dio.dart';
import 'package:equatable/equatable.dart';
import 'package:json_annotation/json_annotation.dart';

import '../api/api_client.dart';
import '../api/api_endpoints.dart';
import '../auth/auth_token_storage.dart';
import '../models/user_model.dart';

part 'auth_service.g.dart';

/// Authentication response from the backend.
@JsonSerializable()
class AuthResponse extends Equatable {
  const AuthResponse({
    this.token,
    required this.user,
    this.refreshToken,
    this.requires2fa = false,
    this.requires2faSetup = false,
    this.tempToken,
  });

  factory AuthResponse.fromJson(Map<String, dynamic> json) =>
      _$AuthResponseFromJson(json);
  Map<String, dynamic> toJson() => _$AuthResponseToJson(this);

  final String? token;

  @JsonKey(name: 'refresh_token')
  final String? refreshToken;

  final UserModel user;
  @JsonKey(name: 'requires_2fa')
  final bool requires2fa;

  /// Whether 2FA setup is required by the tenant (forced enrollment).
  @JsonKey(name: 'requires_2fa_setup')
  final bool requires2faSetup;

  /// Temporary token used during 2FA challenge or forced enrollment.
  @JsonKey(name: 'temp_token')
  final String? tempToken;

  @override
  List<Object?> get props => [token, refreshToken, user, requires2fa, requires2faSetup, tempToken];
}

/// Result wrapper for service operations.
sealed class ServiceResult<T> {
  const ServiceResult();

  W fold<W>(
    W Function(T data) onSuccess,
    W Function(ApiException exception) onFailure,
  ) {
    final self = this;
    if (self is Success<T>) return onSuccess(self.data);
    return onFailure((self as Failure<T>).exception);
  }

  T getOrThrow() {
    final self = this;
    if (self is Success<T>) return self.data;
    throw (self as Failure<T>).exception;
  }

  T? getOrNull() {
    final self = this;
    if (self is Success<T>) return self.data;
    return null;
  }
}

class Success<T> extends ServiceResult<T> {
  const Success(this.data);
  final T data;
}

class Failure<T> extends ServiceResult<T> {
  const Failure(this.exception);
  final ApiException exception;
}

/// Auth service — login, logout, 2FA, password management.
class AuthService {
  AuthService({required this.dio, required this.tokenStorage});

  final Dio dio;
  final AuthTokenStorage tokenStorage;

  /// Login with email + password.
  Future<ServiceResult<AuthResponse>> login({
    required String email,
    required String password,
  }) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.authLogin,
        data: {'email': email, 'password': password},
      );
      return AuthResponse.fromJson(res.data ?? const {});
    });
  }

  /// Verify 2FA challenge (TOTP, email OTP).
  Future<ServiceResult<AuthResponse>> verify2fa({
    required String tempToken,
    required String code,
    String method = 'totp',
  }) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.auth2faVerify,
        data: {'tempToken': tempToken, 'code': code, 'method': method},
      );
      return AuthResponse.fromJson(res.data ?? const {});
    });
  }

  // ── Temp-token 2FA Setup (forced enrollment — no JWT required) ──

  /// Enable 2FA from temp token: generate secret & QR code.
  Future<ServiceResult<Map<String, dynamic>>> enable2faTemp(
    String tempToken,
  ) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.auth2faTempEnable,
        data: {'tempToken': tempToken},
      );
      return res.data ?? {};
    });
  }

  /// Verify 2FA setup + complete login (TOTP).
  Future<ServiceResult<AuthResponse>> verify2faSetupTemp({
    required String tempToken,
    required String secret,
    required String code,
  }) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.auth2faTempVerifySetup,
        data: {'tempToken': tempToken, 'secret': secret, 'code': code},
      );
      return AuthResponse.fromJson(res.data ?? const {});
    });
  }

  /// Request email OTP for 2FA setup (temp token).
  Future<ServiceResult<bool>> requestEmail2faSetupTemp(
    String tempToken,
  ) async {
    return _execute(() async {
      await dio.post(ApiEndpoints.auth2faTempEmailEnableRequest,
          data: {'tempToken': tempToken});
      return true;
    });
  }

  /// Verify email 2FA setup + complete login.
  Future<ServiceResult<AuthResponse>> verifyEmail2faSetupTemp({
    required String tempToken,
    required String code,
  }) async {
    return _execute(() async {
      final res = await dio.post<Map<String, dynamic>>(
        ApiEndpoints.auth2faTempEmailEnableVerify,
        data: {'tempToken': tempToken, 'code': code},
      );
      return AuthResponse.fromJson(res.data ?? const {});
    });
  }

  /// Fetch the current authenticated user.
  Future<ServiceResult<UserModel>> me() async {
    return _execute(() async {
      final res = await dio.get<Map<String, dynamic>>(ApiEndpoints.authMe);
      return UserModel.fromJson(res.data ?? const {});
    });
  }

  /// Logout — clears token locally.
  ///
  /// The backend has no `/api/auth/logout` endpoint, so this is a local-only
  /// operation that removes stored credentials from secure storage.
  Future<void> logout() async {
    await tokenStorage.clear();
  }

  /// Request password reset email.
  Future<ServiceResult<bool>> forgotPassword(String email) async {
    return _execute(() async {
      await dio.post(ApiEndpoints.authForgotPassword, data: {'email': email});
      return true;
    });
  }

  /// Complete password reset with token.
  Future<ServiceResult<bool>> resetPassword({
    required String token,
    required String newPassword,
  }) async {
    return _execute(() async {
      await dio.post(ApiEndpoints.authResetPassword, data: {
        'token': token,
        'password': newPassword,
      });
      return true;
    });
  }

  /// Persist tokens & user info locally.
  Future<void> persistSession(AuthResponse auth) async {
    final t = auth.token;
    if (t == null || t.isEmpty) return; // 2FA challenge — no token yet
    await tokenStorage.save(
      token: t,
      refreshToken: auth.refreshToken,
      userId: auth.user.id,
      tenantId: auth.user.tenantId,
    );
  }

  /// Check if we have a saved session.
  Future<bool> hasSession() async {
    final token = await tokenStorage.readToken();
    if (token == null) return false;
    return token.isNotEmpty;
  }

  Future<ServiceResult<T>> _execute<T>(Future<T> Function() body) async {
    try {
      final result = await body();
      return Success(result);
    } on DioException catch (e) {
      return Failure(ApiException.fromDio(e));
    } catch (e) {
      return Failure(ApiException(message: e.toString()));
    }
  }
}
