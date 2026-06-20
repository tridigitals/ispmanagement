import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../features/auth/auth_loading_screen.dart';
import '../features/auth/login_screen.dart';
import '../features/onboarding/onboarding_screen.dart';
import '../features/home/home_screen.dart';
import '../features/tickets/tickets_screen.dart';
import '../features/tickets/ticket_detail_screen.dart';
import '../services/auth_providers.dart';
import '../services/missing_providers.dart';

/// Build the technician app router. Only /login + /home for Sprint 1;
/// more routes (work order detail, customer view, etc.) added in
/// later sprints as features come online.
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
        path: '/',
        builder: (_, __) => const HomeScreen(),
      ),
      GoRoute(
        path: '/tickets',
        builder: (_, __) => const TicketsScreen(),
      ),
      GoRoute(
        path: '/tickets/:id',
        builder: (_, state) => TicketDetailScreen(
          ticketId: state.pathParameters['id']!,
        ),
      ),
    ],
  );
}