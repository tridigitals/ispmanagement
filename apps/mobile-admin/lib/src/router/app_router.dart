import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:mobile_admin/src/services/auth_providers.dart';
import 'package:mobile_admin/src/features/auth/login_screen.dart';
import 'package:mobile_admin/src/features/home/admin_shell.dart';
import 'package:mobile_admin/src/features/dashboard/dashboard_screen.dart';
import 'package:mobile_admin/src/features/customers/customer_list_screen.dart';
import 'package:mobile_admin/src/features/customers/customer_detail_screen.dart';
import 'package:mobile_admin/src/features/tickets/ticket_list_screen.dart';
import 'package:mobile_admin/src/features/tickets/ticket_detail_screen.dart';
import 'package:mobile_admin/src/features/announcements/announcement_list_screen.dart';
import 'package:mobile_admin/src/features/announcements/announcement_edit_screen.dart';
import 'package:mobile_admin/src/features/settings/admin_settings_screen.dart';
import 'package:mobile_admin/src/features/profile/admin_profile_screen.dart';
import 'package:mobile_admin/src/features/notifications/notification_inbox_screen.dart';

final appRouterProvider = Provider<GoRouter>((ref) {
  final auth = ref.watch(authProvider);

  return GoRouter(
    initialLocation: '/login',
    redirect: (context, state) {
      final isLoggedIn = auth.isAuthenticated;
      final isLoginRoute = state.matchedLocation == '/login';

      if (!isLoggedIn && !isLoginRoute) return '/login';
      if (isLoggedIn && isLoginRoute) return '/';
      return null;
    },
    routes: [
      GoRoute(
        path: '/login',
        builder: (context, state) => const LoginScreen(),
      ),
      ShellRoute(
        builder: (context, state, child) => AdminShell(child: child),
        routes: [
          GoRoute(
            path: '/',
            builder: (context, state) => const DashboardScreen(),
          ),
          GoRoute(
            path: '/customers',
            builder: (context, state) => const CustomerListScreen(),
            routes: [
              GoRoute(
                path: ':id',
                builder: (context, state) => CustomerDetailScreen(
                  customerId: state.pathParameters['id']!,
                ),
              ),
            ],
          ),
          GoRoute(
            path: '/tickets',
            builder: (context, state) => const TicketListScreen(),
            routes: [
              GoRoute(
                path: ':id',
                builder: (context, state) => TicketDetailScreen(
                  ticketId: state.pathParameters['id']!,
                ),
              ),
            ],
          ),
          GoRoute(
            path: '/announcements',
            builder: (context, state) => const AnnouncementListScreen(),
            routes: [
              GoRoute(
                path: 'new',
                builder: (context, state) => const AnnouncementEditScreen(),
              ),
              GoRoute(
                path: ':id',
                builder: (context, state) => AnnouncementEditScreen(
                  announcementId: state.pathParameters['id'],
                ),
              ),
            ],
          ),
          GoRoute(
            path: '/settings',
            builder: (context, state) => const AdminSettingsScreen(),
          ),
          GoRoute(
            path: '/profile',
            builder: (context, state) => const AdminProfileScreen(),
          ),
          GoRoute(
            path: '/notifications',
            builder: (context, state) => const NotificationInboxScreen(),
          ),
        ],
      ),
    ],
  );
});
