import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
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
import '../features/invoices/invoice_detail_screen.dart';
import '../features/notifications/notification_inbox_screen.dart';
import '../features/onboarding/onboarding_screen.dart';
import '../features/permissions/permissions_screen.dart';
import '../features/payments/payment_instruction_screen.dart';
import '../features/payments/payment_screen.dart';
import '../features/payments/payment_webview_screen.dart';
import '../features/profile/change_password_screen.dart';
import '../features/profile/edit_profile_screen.dart';
import '../features/profile/profile_screen.dart';
import '../features/settings/settings_screen.dart';
import '../features/subscriptions/subscription_detail_screen.dart';
import '../features/tickets/new_ticket_screen.dart';
import '../features/tickets/ticket_detail_screen.dart';
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
      final permissionsDone = container.read(permissionsCompletedProvider);
      final loc = state.matchedLocation;
      final isPublic = loc == '/login' || loc == '/forgot-password'
          || loc == '/onboarding' || loc == '/loading' || loc == '/permissions';

      // First-run gate: onboarding
      if (!onboardingDone && loc != '/onboarding') {
        return '/onboarding';
      }

      // Permissions gate: after onboarding, force /permissions
      if (onboardingDone && !permissionsDone && loc != '/permissions') {
        return '/permissions';
      }

      // Auth gate
      if (!loggedIn && !isPublic && loc != '/onboarding' && loc != '/permissions') {
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
            path: 'subscriptions/:id',
            builder: (_, state) =>
                SubscriptionDetailScreen(id: state.pathParameters['id']!),
          ),
          GoRoute(
            path: 'invoices/:id',
            builder: (_, state) =>
                InvoiceDetailScreen(id: state.pathParameters['id']!),
          ),
          GoRoute(
            path: 'tickets/new',
            builder: (_, __) => const NewTicketScreen(),
          ),
          GoRoute(
            path: 'tickets/:id',
            builder: (_, state) =>
                TicketDetailScreen(id: state.pathParameters['id']!),
          ),
          GoRoute(
            path: 'payments/:invoiceId',
            builder: (_, state) => PaymentScreen(
              invoiceId: state.pathParameters['invoiceId']!,
            ),
            routes: [
              GoRoute(
                path: 'webview',
                builder: (_, state) => PaymentWebViewScreen(
                  paymentUrl: state.extra as String,
                  invoiceId: state.pathParameters['invoiceId']!,
                ),
              ),
            ],
          ),
          GoRoute(
            path: 'payments/:invoiceId/:transactionId/instructions',
            builder: (_, state) => PaymentInstructionScreen(
              invoiceId: state.pathParameters['invoiceId']!,
              transactionId: state.pathParameters['transactionId']!,
            ),
          ),
        ],
      ),
    ],
  );
}
