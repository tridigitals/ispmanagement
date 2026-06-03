import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'service_providers.dart';

class AuthState {
  const AuthState({this.user, this.isLoading = false});
  final UserModel? user;
  final bool isLoading;

  bool get isAuthenticated => user != null;
  AuthState copyWith({UserModel? user, bool? isLoading, bool clearUser = false}) {
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
    state = state.copyWith(isLoading: false);
    return res;
  }

  Future<ServiceResult<AuthResponse>> verify2fa({
    required String tempToken,
    required String code,
  }) async {
    state = state.copyWith(isLoading: true);
    final res = await ref.read(authServiceProvider).verify2fa(
          tempToken: tempToken,
          code: code,
        );
    state = state.copyWith(isLoading: false);
    return res;
  }

  /// Persist the auth response (token + user) into local state.
  Future<void> apply(AuthResponse auth) async {
    await ref.read(authServiceProvider).persistSession(auth);
    state = AuthState(user: auth.user);
  }

  /// Hydrate from secure storage on app start.
  Future<void> bootstrap() async {
    final auth = ref.read(authServiceProvider);
    if (!await auth.hasSession()) return;
    final me = await auth.me();
    me.when(
      success: (u) => state = AuthState(user: u),
      failure: (_) => auth.logout(),
    );
  }

  Future<void> logout() async {
    await ref.read(authServiceProvider).logout();
    state = const AuthState(clearUser: true);
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
