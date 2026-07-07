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
      final auth = ref.read(authControllerProvider);
      if (!auth.isAuthenticated) {
        final restored =
            await ref.read(authControllerProvider.notifier).bootstrap();
        if (!restored || !mounted) {
          context.go('/login');
          return;
        }
      }

      if (!mounted) return;

      ref.read(fcmServiceProvider).clearPendingAction();
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
            // ─── Solid purple square logo (no gradient) ───
            Container(
              width: 64,
              height: 64,
              decoration: BoxDecoration(
                color: isp.accent, // solid, no gradient
                borderRadius: BorderRadius.circular(18),
              ),
              alignment: Alignment.center,
              child: const Text(
                'IS',
                style: TextStyle(
                  fontSize: 30,
                  fontWeight: FontWeight.w900,
                  color: Colors.white,
                  letterSpacing: -2,
                ),
              ),
            ),
            const SizedBox(height: 28),
            Text(
              'Memeriksa sesi...',
              style: TextStyle(
                color: isp.textMuted,
                fontSize: 13,
              ),
            ),
            const SizedBox(height: 16),
            SizedBox(
              width: 24,
              height: 24,
              child: CircularProgressIndicator(
                strokeWidth: 2.5,
                valueColor: AlwaysStoppedAnimation<Color>(isp.accent),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
