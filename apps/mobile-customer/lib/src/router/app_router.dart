import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:ui_kit/ui_kit.dart';

import '../features/auth/forgot_password_screen.dart';
import '../features/auth/login_screen.dart';
import '../features/auth/otp_login_screen.dart';
import '../features/auth/otp_verify_screen.dart';
import '../features/auth/register_with_invite_screen.dart';
import '../features/auth/two_factor_enroll_screen.dart';
import '../features/auth/two_factor_verify_screen.dart';
import '../features/contact/contact_screen.dart';
import '../features/faq/faq_screen.dart';
import '../features/home/home_shell.dart';
import '../features/invoices/invoice_detail_screen.dart';
import '../features/notifications/notification_inbox_screen.dart';
import '../features/onboarding/onboarding_screen.dart';
import '../features/payments/payment_instruction_screen.dart';
import '../features/payments/payment_screen.dart';
import '../features/profile/change_password_screen.dart';
import '../features/profile/edit_profile_screen.dart';
import '../features/settings/settings_screen.dart';
import '../features/subscriptions/subscription_detail_screen.dart';
import '../features/tickets/new_ticket_screen.dart';
import '../features/tickets/ticket_detail_screen.dart';
import '../services/auth_providers.dart';
import '../services/feature_providers.dart';
import '../services/notification_service.dart';

GoRouter buildAppRouter({
  required WidgetRef ref,
  required ProviderContainer container,
}) {
  return GoRouter(
    initialLocation: '/',
    refreshListenable: GoRouterRefreshStream(
      container.listen(authControllerProvider, (_, __) {}),
    ),
    redirect: (context, state) async {
      final auth = container.read(authControllerProvider);
      final loggedIn = auth.isAuthenticated;
      final onboardingDone = container
              .read(onboardingCompletedProvider)
              .valueOrNull ??
          true;
      final loc = state.matchedLocation;
      final isPublic = loc == '/login' ||
          loc == '/login/otp' ||
          loc == '/login/otp/verify' ||
          loc == '/register' ||
          loc == '/forgot-password' ||
          loc == '/onboarding';

      // First-run gate
      if (!onboardingDone && loc != '/onboarding') {
        return '/onboarding';
      }

      // Auth gate
      if (!loggedIn && !isPublic && loc != '/onboarding') {
        return '/login';
      }
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
        routes: [
          GoRoute(
            path: 'otp',
            builder: (_, __) => const OtpLoginScreen(),
            routes: [
              GoRoute(
                path: 'verify',
                builder: (_, state) =>
                    OtpVerifyScreen(phone: state.extra as String?),
              ),
            ],
          ),
        ],
      ),
      GoRoute(
        path: '/forgot-password',
        builder: (_, __) => const ForgotPasswordScreen(),
      ),
      GoRoute(
        path: '/register',
        builder: (_, __) => const RegisterWithInviteScreen(),
      ),
      GoRoute(
        path: '/security/2fa/enroll',
        builder: (_, __) => const TwoFactorEnrollScreen(),
      ),
      GoRoute(
        path: '/2fa/verify',
        builder: (_, state) => TwoFactorVerifyScreen(
          pendingToken: state.extra as String,
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
          ),
          GoRoute(
            path: 'payments/:transactionId/instructions',
            builder: (_, state) => PaymentInstructionScreen(
              transactionId: state.pathParameters['transactionId']!,
            ),
          ),
        ],
      ),
    ],
  );
}
