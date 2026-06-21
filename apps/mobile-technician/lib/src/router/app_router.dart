import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../features/auth/auth_loading_screen.dart';
import '../features/auth/forgot_password_screen.dart';
import '../features/auth/login_screen.dart';
import '../features/home/home_shell.dart';
import '../features/onboarding/onboarding_screen.dart';
import '../features/profile/edit_profile_screen.dart';
import '../features/profile/change_password_screen.dart';
import '../features/tickets/ticket_detail_screen.dart';
import '../features/settings/settings_screen.dart';
import '../features/faq/faq_screen.dart';
import '../features/contact/contact_screen.dart';
import '../features/notifications/notification_inbox_screen.dart';
import '../services/auth_providers.dart';
import '../services/missing_providers.dart';

GoRouter buildAppRouter({
  required WidgetRef ref,
  required ProviderContainer container,
  GlobalKey<NavigatorState>? navigatorKey,
}) {
  return GoRouter(
    navigatorKey: navigatorKey,
    initialLocation: '/',
    refreshListenable: container.read(authStateProvider),
    redirect: (context, state) async {
      final auth = container.read(authControllerProvider);
      final loggedIn = auth.isAuthenticated;
      final onboardingDone = container.read(onboardingCompletedProvider);
      final loc = state.matchedLocation;
      final isPublic =
          loc == '/login' || loc == '/forgot-password' || loc == '/onboarding' || loc == '/loading';

      // First-run gate
      if (!onboardingDone && loc != '/onboarding') {
        return '/onboarding';
      }

      // Auth gate
      if (!loggedIn && !isPublic && loc != '/onboarding') {
        return '/login';
      }
      // Don't redirect /loading away — let AuthLoadingScreen control nav.
      if (loggedIn && (loc == '/login' || loc == '/onboarding')) {
        return '/';
      }
      return null;
    },
    routes: [
      GoRoute(
        path: '/onboarding',
        builder: (_, __) => const OnboardingScreen(),
      ),
      GoRoute(
        path: '/login',
        builder: (_, __) => const LoginScreen(),
      ),
      GoRoute(
        path: '/loading',
        builder: (_, __) => const AuthLoadingScreen(),
      ),
      GoRoute(
        path: '/forgot-password',
        builder: (_, __) => const ForgotPasswordScreen(),
      ),
      GoRoute(
        path: '/',
        builder: (_, __) => const HomeShell(),
        routes: [
          GoRoute(
            path: 'profile',
            builder: (_, __) => const Scaffold(body: Center(child: Text('Profile'))),
          ),
          GoRoute(
            path: 'settings',
            builder: (_, __) => const SettingsScreen(),
          ),
          GoRoute(
            path: 'change-password',
            builder: (_, __) => const ChangePasswordScreen(),
          ),
          GoRoute(
            path: 'edit-profile',
            builder: (_, __) => const EditProfileScreen(),
          ),
          GoRoute(
            path: 'notifications',
            builder: (_, __) => const NotificationInboxScreen(),
          ),
          GoRoute(
            path: 'faq',
            builder: (_, __) => const FaqScreen(),
          ),
          GoRoute(
            path: 'contact',
            builder: (_, __) => const ContactScreen(),
          ),
          GoRoute(
            path: 'tickets/:id',
            builder: (_, state) => TicketDetailScreen(
              ticketId: state.pathParameters['id']!,
            ),
          ),
        ],
      ),
    ],
  );
}
