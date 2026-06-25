import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';
import '../features/announcements/announcement_detail_screen.dart';
import '../features/announcements/announcements_screen.dart';
import '../features/auth/forgot_password_screen.dart';
import '../features/auth/auth_loading_screen.dart';
import '../features/auth/login_screen.dart';
import '../features/auth/two_factor_enroll_screen.dart';
import '../features/auth/two_factor_verify_screen.dart';
import '../features/contact/contact_screen.dart';
import '../features/faq/faq_screen.dart';
import '../features/home/home_shell.dart';
import '../features/notifications/notification_inbox_screen.dart';
import '../features/onboarding/onboarding_screen.dart';
import '../features/permissions/permissions_screen.dart';
import '../features/profile/change_password_screen.dart';
import '../features/profile/edit_profile_screen.dart';
import '../features/profile/profile_screen.dart';
import '../features/settings/settings_screen.dart';

import '../features/tickets/ticket_detail_screen.dart';
import '../features/work_orders/work_order_detail_screen.dart';
import '../services/auth_providers.dart';
import '../services/missing_providers.dart';

GoRouter buildAppRouter({
  required WidgetRef ref,
  required ProviderContainer container,
  GlobalKey<NavigatorState>? navigatorKey,
}) {
  return GoRouter(
    navigatorKey: navigatorKey,
    initialLocation: '/loading',
    refreshListenable: container.read(authStateProvider),
    redirect: (context, state) async {
      final auth = container.read(authControllerProvider);
      final loggedIn = auth.isAuthenticated;
      final onboardingDone = container.read(onboardingCompletedProvider);
      final loc = state.matchedLocation;

      // Permissions is a normal public route — onboarding navigates to it via
      // context.go() directly, no redirect needed. Checking permissions_completed
      // here causes a redirect loop because onboardingCompletedProvider hasn't
      // fired notifyListeners() yet when GoRouter re-evaluates after the go().
      final isPublic = loc == '/login' || loc == '/forgot-password'
          || loc == '/onboarding' || loc == '/loading' || loc == '/permissions';

      // First-run gate: onboarding
      if (!onboardingDone && loc != '/onboarding') {
        return '/onboarding';
      }

      // Auth gate
      if (!loggedIn && !isPublic) {
        return '/login';
      }
      if (loggedIn && (loc == '/login' || loc == '/onboarding' || loc == '/permissions')) {
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
        path: '/permissions',
        builder: (_, __) => const PermissionsScreen(),
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
        path: '/security/2fa/enroll',
        builder: (_, __) => const TwoFactorEnrollScreen(),
      ),
      GoRoute(
        path: '/2fa/verify',
        redirect: (context, state) {
          if (state.extra == null) return '/login';
          return null;
        },
        builder: (_, state) => TwoFactorVerifyScreen(
          pendingToken: (state.extra as String?) ?? '',
        ),
      ),
      GoRoute(
        path: '/',
        builder: (_, __) => const HomeShell(),
        routes: [
          GoRoute(
            path: 'profile',
            builder: (_, __) => const ProfileScreen(),
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
            path: 'announcements',
            builder: (_, __) => const AnnouncementsScreen(),
          ),
          GoRoute(
            path: 'announcements/:id',
            builder: (_, state) => AnnouncementDetailScreen(
              id: state.pathParameters['id']!,
            ),
          ),

          GoRoute(
            path: 'tickets/:id',
            builder: (_, state) =>
                TicketDetailScreen(id: state.pathParameters['id']!),
          ),
          GoRoute(
            path: 'work-orders/:id',
            builder: (_, state) =>
                WorkOrderDetailScreen(id: state.pathParameters['id']!),
          ),
        ],
      ),
    ],
  );
}
