import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:package_info_plus/package_info_plus.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/app_config.dart';
import '../../services/auth_providers.dart';
import '../../services/notifications_providers.dart';
import '../../services/settings_providers.dart';

class ProfileScreen extends ConsumerWidget {
  const ProfileScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final user = ref.watch(currentUserProvider);
    final unread = ref.watch(unreadNotificationsCountProvider).valueOrNull ?? 0;
    final apiBaseUrl = ref.watch(appConfigProvider).apiBaseUrl;
    final themeMode = ref.watch(themeModeProvider);

    return Scaffold(
      backgroundColor: isp.background,
      appBar: AppBar(
        title: Text(l10n.profile),
        centerTitle: false,
      ),
      body: ListView(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        children: [
          const SizedBox(height: 24),
          // Avatar row — accent glow ring
          Center(
            child: Container(
              width: 100,
              height: 100,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                border: Border.all(color: isp.accent, width: 2),
              ),
              child: CircleAvatar(
                radius: 47,
                backgroundColor: isp.surfaceElevated,
                backgroundImage: user?.avatarUrl != null
                    ? NetworkImage(
                        _buildAbsoluteUrl(apiBaseUrl, user!.avatarUrl!))
                    : null,
                child: user?.avatarUrl == null
                    ? Icon(Icons.person, size: 40, color: isp.accent)
                    : null,
              ),
            ),
          ),
          const SizedBox(height: 12),
          Text(
            user?.name ?? '—',
            textAlign: TextAlign.center,
            style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w800),
          ),
          if (user?.email != null)
            Text(
              user!.email,
              textAlign: TextAlign.center,
              style: TextStyle(color: isp.textMuted, fontSize: 13),
            ),
          if (user?.phone != null && user!.phone?.isNotEmpty == true)
            Text(
              user.phone!,
              textAlign: TextAlign.center,
              style: TextStyle(color: isp.textMuted, fontSize: 13),
            ),
          const SizedBox(height: 28),

          // Order: Edit Profil, FAQ, Hubungi Kami, Notifikasi, Ubah Kata Sandi, Tema
          _buildSection(isp, [
            _TintedTile(
              icon: Icons.person_outline,
              iconBg: isp.accent,
              title: l10n.editProfile,
              onTap: () => GoRouter.of(context).push('/edit-profile'),
            ),
            _TintedTile(
              icon: Icons.help_outline,
              iconBg: isp.success,
              title: l10n.faq,
              onTap: () => GoRouter.of(context).push('/faq'),
            ),
            _TintedTile(
              icon: Icons.support_agent_outlined,
              iconBg: isp.info,
              title: l10n.contactUs,
              onTap: () => GoRouter.of(context).push('/contact'),
            ),
            _TintedTile(
              icon: Icons.notifications_outlined,
              iconBg: isp.warning,
              title: l10n.notifications,
              badge: unread > 0 ? '$unread' : null,
              onTap: () => GoRouter.of(context).push('/notifications'),
            ),
            _TintedTile(
              icon: Icons.lock_outline,
              iconBg: isp.info,
              title: l10n.changePassword,
              onTap: () => GoRouter.of(context).push('/change-password'),
            ),
          ]),

          // Theme toggle
          _buildThemeToggle(context, isp, themeMode, ref),

          // Settings + Logout
          _buildSection(isp, [
            _TintedTile(
              icon: Icons.settings,
              iconBg: isp.textMuted,
              title: 'Pengaturan',
              onTap: () => GoRouter.of(context).push('/settings'),
            ),
            _TintedTile(
              icon: Icons.logout,
              iconBg: isp.danger,
              title: l10n.logout,
              isDestructive: true,
              onTap: () => ref.read(authControllerProvider.notifier).logout(),
            ),
          ]),

          const SizedBox(height: 16),
          _VersionLabel(),
          const SizedBox(height: 48),
        ],
      ),
    );
  }

  String _buildAbsoluteUrl(String baseUrl, String url) {
    if (url.startsWith('http://') || url.startsWith('https://')) return url;
    return '$baseUrl$url';
  }
}

/// Grouped tinted card — Apple settings style
Widget _buildSection(IspThemeColors isp, List<Widget> tiles) {
  return Padding(
    padding: const EdgeInsets.only(bottom: 12),
    child: Container(
      decoration: BoxDecoration(
        color: isp.surface,
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: isp.border, width: 1.5),
        boxShadow: [
          BoxShadow(color: isp.border.withOpacity(0.3), offset: Offset(2, 2))
        ],
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        children: [
          for (var i = 0; i < tiles.length; i++) ...[
            tiles[i],
            if (i < tiles.length - 1)
              Divider(height: 1, indent: 56, color: isp.borderSubtle),
          ],
        ],
      ),
    ),
  );
}

class _TintedTile extends StatelessWidget {
  const _TintedTile({
    required this.icon,
    required this.iconBg,
    required this.title,
    this.badge,
    this.isDestructive = false,
    required this.onTap,
  });

  final IconData icon;
  final Color iconBg;
  final String title;
  final String? badge;
  final bool isDestructive;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        child: Row(
          children: [
            // Tinted icon container
            Container(
              width: 32,
              height: 32,
              decoration: BoxDecoration(
                color: iconBg.withOpacity(0.15),
                borderRadius: BorderRadius.circular(7),
              ),
              alignment: Alignment.center,
              child: Icon(icon, size: 17, color: iconBg),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                title,
                style: TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                  color: isDestructive ? isp.danger : isp.textPrimary,
                ),
              ),
            ),
            if (badge != null)
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
                decoration: BoxDecoration(
                  color: isp.danger,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Text(
                  badge!,
                  style: const TextStyle(
                      color: Colors.white,
                      fontSize: 10,
                      fontWeight: FontWeight.w700),
                ),
              ),
            const SizedBox(width: 8),
            Icon(Icons.chevron_right, size: 18, color: isp.textMuted),
          ],
        ),
      ),
    );
  }
}

Widget _buildThemeToggle(
  BuildContext context,
  IspThemeColors isp,
  ThemeMode themeMode,
  WidgetRef ref,
) {
  return Padding(
    padding: const EdgeInsets.only(bottom: 12),
    child: Container(
      decoration: BoxDecoration(
        color: isp.surface,
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: isp.border, width: 1.5),
        boxShadow: [
          BoxShadow(color: isp.border.withOpacity(0.3), offset: Offset(2, 2))
        ],
      ),
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Row(
        children: [
          Container(
            width: 32,
            height: 32,
            decoration: BoxDecoration(
              color: isp.accent.withOpacity(0.15),
              borderRadius: BorderRadius.circular(7),
            ),
            alignment: Alignment.center,
            child: Icon(
              themeMode == ThemeMode.dark ? Icons.dark_mode : Icons.light_mode,
              size: 17,
              color: isp.accent,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Text('Mode Gelap',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600)),
          ),
          // Compact pill toggle
          Container(
            decoration: BoxDecoration(
              color: isp.surfaceTertiary,
              borderRadius: BorderRadius.circular(20),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                _toggleBtn(
                    Icons.light_mode,
                    themeMode == ThemeMode.light,
                    isp,
                    () => ref
                        .read(themeModeProvider.notifier)
                        .set(ThemeMode.light)),
                _toggleBtn(
                    Icons.brightness_auto,
                    themeMode == ThemeMode.system,
                    isp,
                    () => ref
                        .read(themeModeProvider.notifier)
                        .set(ThemeMode.system)),
                _toggleBtn(
                    Icons.dark_mode,
                    themeMode == ThemeMode.dark,
                    isp,
                    () => ref
                        .read(themeModeProvider.notifier)
                        .set(ThemeMode.dark)),
              ],
            ),
          ),
        ],
      ),
    ),
  );
}

Widget _toggleBtn(
    IconData icon, bool selected, IspThemeColors isp, VoidCallback onTap) {
  return GestureDetector(
    onTap: onTap,
    child: AnimatedContainer(
      duration: const Duration(milliseconds: 200),
      padding: const EdgeInsets.all(7),
      decoration: BoxDecoration(
        color: selected ? isp.accent : Colors.transparent,
        shape: BoxShape.circle,
      ),
      child:
          Icon(icon, size: 16, color: selected ? Colors.white : isp.textMuted),
    ),
  );
}

class _VersionLabel extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return FutureBuilder<String>(
      future: PackageInfo.fromPlatform()
          .then((p) => 'v${p.version}+${p.buildNumber}'),
      builder: (_, snap) => Center(
        child: Text(
          snap.data ?? 'v...',
          style: TextStyle(color: isp.textMuted, fontSize: 11),
        ),
      ),
    );
  }
}
