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

  /// Temporary token used during 2FA challenge.
  @JsonKey(name: 'temp_token')
  final String? tempToken;

  @override
  List<Object?> get props => [token, refreshToken, user, requires2fa, tempToken];
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
        data: {'temp_token': tempToken, 'code': code, 'method': method},
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
