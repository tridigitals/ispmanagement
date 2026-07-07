import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../services/missing_providers.dart';

class OnboardingScreen extends ConsumerStatefulWidget {
  const OnboardingScreen({super.key});
  @override
  ConsumerState<OnboardingScreen> createState() => _State();
}

class _State extends ConsumerState<OnboardingScreen> {
  final _controller = PageController();
  int _page = 0;

  static final _pages = <_OnboardPage>[
    const _OnboardPage(
      icon: Icons.wifi_tethering,
      title: 'Internet Cepat',
      body:
          'Pantau kecepatan, bayar tagihan, dan laporkan gangguan — semua dari satu aplikasi.',
      isPremium: true,
    ),
    const _OnboardPage(
      icon: Icons.receipt_long,
      title: 'Bayar Tagihan Mudah',
      body:
          'Virtual Account, e-wallet, QRIS, dan kartu kredit. Bayar di mana saja.',
    ),
    const _OnboardPage(
      icon: Icons.headset_mic,
      title: 'Lapor Gangguan Cepat',
      body: 'Buat tiket dukungan dan lacak status perbaikan secara real-time.',
    ),
    const _OnboardPage(
      icon: Icons.notifications_active_rounded,
      title: 'Tetap Terinformasi',
      body:
          'Aktifkan notifikasi untuk menerima pengingat tagihan, update tiket, dan informasi penting dari ISP.',
      isLast: true,
    ),
  ];

  Future<void> _completeOnboarding() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setBool('onboarding_completed', true);
    } catch (e) {
      debugPrint('[onboarding] SharedPreferences error: $e');
    }
    if (!mounted) return;
    ref.read(onboardingCompletedProvider.notifier).state = true;
    context.go('/permissions');
  }

  Future<void> _next() async {
    if (_page < _pages.length - 1) {
      await _controller.nextPage(
        duration: const Duration(milliseconds: 280),
        curve: Curves.easeOut,
      );
    } else {
      await _completeOnboarding();
    }
  }

  Future<void> _skip() async {
    await _completeOnboarding();
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final isLast = _page == _pages.length - 1;

    return Scaffold(
      backgroundColor: isp.background,
      body: SafeArea(
        child: Column(
          children: [
            Align(
              alignment: Alignment.topRight,
              child: TextButton(
                onPressed: _skip,
                style: TextButton.styleFrom(
                  foregroundColor: isp.textMuted,
                ),
                child: const Text(
                  'Lewati',
                  style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                ),
              ),
            ),
            Expanded(
              child: PageView.builder(
                controller: _controller,
                itemCount: _pages.length,
                onPageChanged: (i) => setState(() => _page = i),
                itemBuilder: (_, i) => _pages[i],
              ),
            ),
            const SizedBox(height: 12),
            // ─── Accent dot indicators ───
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: List.generate(
                _pages.length,
                (i) => AnimatedContainer(
                  duration: const Duration(milliseconds: 200),
                  margin: const EdgeInsets.symmetric(horizontal: 4),
                  width: _page == i ? 24 : 8,
                  height: 8,
                  decoration: BoxDecoration(
                    color: _page == i ? isp.accent : isp.border,
                    borderRadius: BorderRadius.circular(IspRadii.pill),
                  ),
                ),
              ),
            ),
            const SizedBox(height: 28),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 28),
              child: _NeubrutalistAccentButton(
                label: isLast ? 'Aktifkan' : 'Lanjutkan',
                loading: false,
                onTap: _next,
              ),
            ),
            const SizedBox(height: 12),
          ],
        ),
      ),
    );
  }
}

class _OnboardPage extends StatelessWidget {
  const _OnboardPage({
    required this.icon,
    required this.title,
    required this.body,
    this.isLast = false,
    this.isPremium = false,
  });
  final IconData icon;
  final String title;
  final String body;
  final bool isLast;
  final bool isPremium;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 32),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          // ─── Premium slide: larger icon with glow ───
          Container(
            width: isPremium ? 140.0 : 120.0,
            height: isPremium ? 140.0 : 120.0,
            decoration: BoxDecoration(
              color: isp.accent.withOpacity(0.12),
              shape: BoxShape.circle,
              boxShadow: isPremium
                  ? [
                      BoxShadow(
                        color: isp.accent.withOpacity(0.25),
                        blurRadius: 40,
                        spreadRadius: 8,
                      ),
                    ]
                  : null,
            ),
            child: Icon(icon,
                size: isPremium ? 64 : 48, color: isp.accentLight),
          ),
          const SizedBox(height: 32),
          Text(
            title,
            style: TextStyle(
              fontSize: isPremium ? 28 : 22,
              fontWeight: FontWeight.w900,
              letterSpacing: -1,
              color: isp.textPrimary,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 12),
          Text(
            body,
            style: TextStyle(
              color: isp.textMuted,
              fontSize: 14,
              height: 1.6,
            ),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}

class _NeubrutalistAccentButton extends StatelessWidget {
  const _NeubrutalistAccentButton({
    required this.label,
    required this.loading,
    required this.onTap,
  });
  final String label;
  final bool loading;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return GestureDetector(
      onTap: loading ? null : onTap,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(vertical: 14),
        decoration: BoxDecoration(
          color: isp.accent,
          border: Border.all(width: 1.5, color: isp.accent),
          borderRadius: BorderRadius.circular(IspRadii.md),
          boxShadow: [
            BoxShadow(
              offset: const Offset(3, 3),
              blurRadius: 0,
              color: isp.accent.withOpacity(0.3),
            ),
          ],
        ),
        child: Center(
          child: loading
              ? SizedBox(
                  width: 18,
                  height: 18,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    valueColor: AlwaysStoppedAnimation<Color>(isp.textInverse),
                  ),
                )
              : Text(
                  label,
                  style: const TextStyle(
                    color: Colors.white,
                    fontSize: 14,
                    fontWeight: FontWeight.w700,
                  ),
                ),
        ),
      ),
    );
  }
}
