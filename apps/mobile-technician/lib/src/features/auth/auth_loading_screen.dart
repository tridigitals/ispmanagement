import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../services/auth_providers.dart';
import '../../services/fcm_service.dart';

/// Loading screen shown right after login before navigating to home.
///
/// Only verifies token exists + session valid (fast — one API call).
/// Home data loading happens naturally via HomeTab's own providers.
class AuthLoadingScreen extends ConsumerStatefulWidget {
  const AuthLoadingScreen({super.key});

  @override
  ConsumerState<AuthLoadingScreen> createState() => _AuthLoadingScreenState();
}

class _AuthLoadingScreenState extends ConsumerState<AuthLoadingScreen> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _verifyAndGo());
  }

  Future<void> _verifyAndGo() async {
    try {
      // Fast check: is there a session token?
      final auth = ref.read(authControllerProvider);
      if (!auth.isAuthenticated) {
        // State not ready yet — try bootstrap from storage
        final restored = await ref.read(authControllerProvider.notifier).bootstrap();
        if (!restored || !mounted) {
          context.go('/login');
          return;
        }
      }

      if (!mounted) return;

      // Navigate to home — data loads naturally in HomeTab
      ref.read(fcmServiceProvider).clearPendingAction();

      // Kick off FCM token registration (fire-and-forget, idempotent).
      // This covers the case where the auth state transition happened
      // before the ref.listen in app.dart was registered (cold start
      // with valid session), and the delayed bootstrap hasn't fired yet.
      ref.read(fcmServiceProvider).init(force: true);

      context.go('/');
    } catch (e) {
      if (!mounted) return;
      debugPrint('[auth] loading screen error: $e');
      context.go('/login');
    }
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;

    return Scaffold(
      backgroundColor: isp.background,
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 80,
              height: 80,
              decoration: BoxDecoration(
                color: isp.accent.withOpacity(0.1),
                borderRadius: BorderRadius.circular(20),
              ),
              child: Icon(
                Icons.wifi_rounded,
                size: 40,
                color: isp.accent,
              ),
            ),
            const SizedBox(height: 32),
            const SizedBox(
              width: 28,
              height: 28,
              child: CircularProgressIndicator(strokeWidth: 3),
            ),
          ],
        ),
      ),
    );
  }
}
