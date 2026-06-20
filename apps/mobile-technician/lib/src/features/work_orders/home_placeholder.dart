import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../services/auth_providers.dart';
import '../../theme/app_theme.dart';

/// Placeholder home screen for Sprint 1.
///
/// Shows technician identity + logout so we can verify the auth flow
/// end-to-end (login → bootstrap → home → logout → login) before
/// Sprint 2 builds the real bottom nav + work order list.
class HomePlaceholder extends ConsumerWidget {
  const HomePlaceholder({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final user = ref.watch(currentUserProvider);

    return Scaffold(
      backgroundColor: AppColors.bg,
      appBar: AppBar(
        backgroundColor: AppColors.accent,
        foregroundColor: Colors.white,
        title: const Text('ISP Technician'),
        actions: [
          IconButton(
            icon: const Icon(Icons.logout),
            tooltip: 'Logout',
            onPressed: () async {
              await ref.read(authControllerProvider.notifier).logout(force: true);
              if (context.mounted) context.go('/login');
            },
          ),
        ],
      ),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 96,
                height: 96,
                decoration: BoxDecoration(
                  color: AppColors.accent.withOpacity(0.1),
                  shape: BoxShape.circle,
                ),
                child: const Icon(
                  Icons.engineering_rounded,
                  size: 56,
                  color: AppColors.accent,
                ),
              ),
              const SizedBox(height: 24),
              Text(
                'Login berhasil',
                style: TextStyle(
                  fontSize: 22,
                  fontWeight: FontWeight.w700,
                  color: isp.textPrimary,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                'Selamat datang, ${user?.name ?? "Teknisi"}',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 14, color: isp.textSecondary),
              ),
              const SizedBox(height: 32),
              Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: AppColors.surface,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: AppColors.border),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        const Icon(Icons.badge_outlined, color: AppColors.accent, size: 20),
                        const SizedBox(width: 8),
                        Text(
                          'Akun',
                          style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.w600,
                            color: isp.textMuted,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    Text(
                      user?.email ?? '-',
                      style: TextStyle(fontSize: 14, color: isp.textPrimary),
                    ),
                    Text(
                      'Role: ${user?.role ?? '-'}',
                      style: TextStyle(fontSize: 13, color: isp.textSecondary),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 24),
              Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: AppColors.actionOrange.withOpacity(0.08),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: AppColors.actionOrange.withOpacity(0.3)),
                ),
                child: Row(
                  children: [
                    const Icon(Icons.handyman_outlined, color: AppColors.actionOrange),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        'Sprint 1 selesai. Sprint 2 akan tambah:\n'
                        '• Bottom nav 4 tab\n'
                        '• Jadwal Pekerjaan\n'
                        '• Notifikasi\n'
                        '• Akun',
                        style: TextStyle(fontSize: 13, color: isp.textPrimary),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}