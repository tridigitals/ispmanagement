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
    _OnboardPage(
      icon: Icons.engineering,
      title: 'Aplikasi Teknisi',
      body:
          'Kelola tiket, pantau jadwal, dan selesaikan pekerjaan langsung dari lapangan.',
      color: const Color(0xFF1565C0), // engineer blue
    ),
    _OnboardPage(
      icon: Icons.assignment_turned_in,
      title: 'Kelola Tiket',
      body:
          'Terima tiket, update status, upload foto bukti, dan ambil tanda tangan pelanggan.',
      color: const Color(0xFF22C55E), // success
    ),
    _OnboardPage(
      icon: Icons.location_on,
      title: 'Tracking Lokasi',
      body:
          'Otomatis kirim lokasi ke server sehingga admin dapat memantau posisi teknisi secara real-time.',
      color: const Color(0xFFF59E0B), // warning
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
    context.go('/login');
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
    final isLast = _page == _pages.length - 1;
    return Scaffold(
      body: SafeArea(
        child: Column(
          children: [
            Align(
              alignment: Alignment.topRight,
              child: TextButton(
                onPressed: _skip,
                child: const Text('Lewati'),
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
            const SizedBox(height: 8),
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
                    color:
                        _page == i ? context.isp.accent : context.isp.surfaceTertiary,
                    borderRadius: BorderRadius.circular(IspRadii.pill),
                  ),
                ),
              ),
            ),
            const SizedBox(height: 24),
            Padding(
              padding: const EdgeInsets.all(IspSpacing.lg),
              child: IspPrimaryButton(
                label: isLast ? 'Mulai' : 'Lanjut',
                icon: isLast ? Icons.check : Icons.arrow_forward,
                onPressed: _next,
              ),
            ),
            const SizedBox(height: 8),
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
    required this.color,
  });
  final IconData icon;
  final String title;
  final String body;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: IspSpacing.xl),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            width: 160,
            height: 160,
            decoration: BoxDecoration(
              color: color.withOpacity(0.12),
              shape: BoxShape.circle,
            ),
            child: Icon(icon, size: 80, color: color),
          ),
          const SizedBox(height: 32),
          Text(
            title,
            style: const TextStyle(
              fontSize: 22,
              fontWeight: FontWeight.w700,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 12),
          Text(
            body,
            style: TextStyle(
              color: isp.textMuted,
              fontSize: 14,
            ),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}
