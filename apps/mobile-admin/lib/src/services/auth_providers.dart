import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app_config.dart';
import 'service_providers.dart';

class AuthState {
  final String? token;
  final Map<String, dynamic>? user;
  final bool isLoading;
  final String? error;

  const AuthState({this.token, this.user, this.isLoading = false, this.error});

  bool get isAuthenticated => token != null;
  String? get role => user?['role'] as String?;
  String? get tenantId => user?['tenant_id'] as String?;

  AuthState copyWith({
    String? token,
    Map<String, dynamic>? user,
    bool? isLoading,
    String? error,
  }) {
    return AuthState(
      token: token ?? this.token,
      user: user ?? this.user,
      isLoading: isLoading,
      error: error,
    );
  }
}

class AuthNotifier extends StateNotifier<AuthState> {
  final ApiClient _api;
  final Ref _ref;

  AuthNotifier(this._api, this._ref) : super(const AuthState());

  Future<void> login(String email, String password) async {
    state = state.copyWith(isLoading: true, error: null);
    try {
      final res = await _api.post('/auth/login', data: {
        'email': email,
        'password': password,
      });
      final token = res.data['token'] as String;
      final user = res.data['user'] as Map<String, dynamic>;

      // Persist token via AuthTokenStorage so the Dio interceptor picks
      // it up on subsequent requests. Failure here is non-fatal —
      // admin is still authenticated for this session.
      try {
        await _ref.read(tokenStorageProvider).saveToken(token);
      } catch (_) {/* secure storage may fail on emulator */}

      state = AuthState(token: token, user: user);
    } on Exception catch (e) {
      state = AuthState(error: e.toString());
    }
  }

  void logout() {
    try {
      _ref.read(tokenStorageProvider).deleteToken();
    } catch (_) {}
    state = const AuthState();
  }
}

final apiClientProvider = Provider<ApiClient>((ref) {
  final config = ref.watch(appConfigProvider);
  return ApiClient(baseUrl: config.apiBaseUrl);
});

final authProvider = StateNotifierProvider<AuthNotifier, AuthState>((ref) {
  final api = ref.watch(apiClientProvider);
  return AuthNotifier(api, ref);
});
