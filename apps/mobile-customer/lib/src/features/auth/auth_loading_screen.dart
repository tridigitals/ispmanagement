import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/app_config.dart';
import '../../services/auth_providers.dart';
import '../../services/fcm_service.dart';
import '../../services/missing_providers.dart';
import '../../services/notifications_providers.dart';
import '../../services/service_providers.dart';

/// Loading screen shown right after login before navigating to home.
///
/// Prevents 401 errors on home screen by ensuring:
/// 1. Token is fully persisted to storage (in-memory cache + native)
/// 2. User session is verified via /api/auth/me
/// 3. Initial data (subscriptions, invoices) are pre-fetched
///
/// Once all data is ready, navigates to home (replaces this route).
class AuthLoadingScreen extends ConsumerStatefulWidget {
  const AuthLoadingScreen({super.key});

  @override
  ConsumerState<AuthLoadingScreen> createState() => _AuthLoadingScreenState();
}

class _AuthLoadingScreenState extends ConsumerState<AuthLoadingScreen> {
  String _status = '';
  bool _hasError = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _initialize());
  }

  Future<void> _initialize() async {
    try {
      // Step 1: Verify token is persisted in storage
      _setStatus('Memverifikasi sesi...');
      final tokenStorage = ref.read(tokenStorageProvider);
      final token = await tokenStorage.readToken();
      if (token == null || token.isEmpty) {
        _failWith('Gagal menyimpan token sesi. Silakan coba login ulang.');
        return;
      }

      // Step 2: Verify session is valid via /me
      _setStatus('Memverifikasi akun...');
      final authSvc = ref.read(authServiceProvider);
      final me = await authSvc.me();
      switch (me) {
        case Success(:final data):
          if (!data.isCustomer) {
            _failWith('Akun ini bukan akun pelanggan.');
            return;
          }
        case Failure():
          _failWith('Sesi tidak valid. Silakan login ulang.');
          return;
      }

      // Step 3: Pre-fetch home screen data
      _setStatus('Memuat data tagihan...');
      final invoices = ref.read(myInvoicesProvider.future);
      final subs = ref.read(mySubscriptionsProvider.future);
      final unread = ref.read(unreadNotificationsCountProvider.future);

      await Future.wait([invoices, subs, unread], eagerError: false);

      if (!mounted) return;

      // Step 4: Navigate to home
      _setStatus('Siap!');
      // Brief delay so user sees "Siap!" momentarily
      await Future.delayed(const Duration(milliseconds: 300));

      if (!mounted) return;
      ref.read(fcmServiceProvider).clearPendingAction();
      context.go('/');
    } catch (e) {
      if (!mounted) return;
      _failWith('Terjadi kesalahan: $e');
    }
  }

  void _setStatus(String msg) {
    if (!mounted) return;
    setState(() {
      _status = msg;
      _hasError = false;
    });
  }

  void _failWith(String msg) {
    if (!mounted) return;
    setState(() {
      _status = msg;
      _hasError = true;
    });
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Scaffold(
      backgroundColor: isp.background,
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // App logo / icon
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

              // Status message
              Text(
                _status,
                textAlign: TextAlign.center,
                style: TextStyle(
                  fontSize: 16,
                  color: _hasError ? isp.danger : isp.textSecondary,
                  fontWeight: _hasError ? FontWeight.w600 : FontWeight.normal,
                ),
              ),

              const SizedBox(height: 24),

              // Loading indicator or retry button
              if (_hasError)
                FilledButton.icon(
                  onPressed: () {
                    setState(() => _hasError = false);
                    _initialize();
                  },
                  icon: const Icon(Icons.refresh),
                  label: Text('Coba Lagi'),
                )
              else
                const SizedBox(
                  width: 28,
                  height: 28,
                  child: CircularProgressIndicator(strokeWidth: 3),
                ),
            ],
          ),
        ),
      ),
    );
  }
}
