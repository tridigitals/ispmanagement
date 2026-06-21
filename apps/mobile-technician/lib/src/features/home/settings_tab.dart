import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../services/settings_providers.dart';

/// Settings tab — profile info, theme toggle, language, logout.
class SettingsTab extends ConsumerWidget {
  const SettingsTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final user = ref.watch(currentUserProvider);
    final themeMode = ref.watch(themeModeProvider);
    final locale = ref.watch(localeProvider);

    return ListView(
      padding: const EdgeInsets.symmetric(vertical: 8),
      children: [
        // Profile card
        Card(
          margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
          ),
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                CircleAvatar(
                  radius: 28,
                  backgroundColor: isp.accent.withOpacity(0.15),
                  child: Text(
                    (user?.name ?? 'T')[0].toUpperCase(),
                    style: TextStyle(
                      fontSize: 24,
                      fontWeight: FontWeight.bold,
                      color: isp.accent,
                    ),
                  ),
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        user?.name ?? 'Teknisi',
                        style: TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.w600,
                          color: isp.textPrimary,
                        ),
                      ),
                      const SizedBox(height: 4),
                      Text(
                        user?.email ?? '',
                        style: TextStyle(
                          fontSize: 13,
                          color: isp.textMuted,
                        ),
                      ),
                      if (user?.phone != null && user!.phone!.isNotEmpty)
                        Padding(
                          padding: const EdgeInsets.only(top: 2),
                          child: Text(
                            user.phone!,
                            style: TextStyle(
                              fontSize: 13,
                              color: isp.textMuted,
                            ),
                          ),
                        ),
                    ],
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.edit_outlined),
                  onPressed: () => context.push('/edit-profile'),
                  tooltip: 'Edit Profil',
                ),
              ],
            ),
          ),
        ),

        const SizedBox(height: 8),

        // Appearance section
        const _SectionHeader(title: 'Tampilan'),
        _SettingsTile(
          icon: Icons.dark_mode_outlined,
          title: 'Mode Gelap',
          trailing: SegmentedButton<ThemeMode>(
            segments: [
              ButtonSegment(
                value: ThemeMode.light,
                icon: const Icon(Icons.light_mode, size: 16),
              ),
              ButtonSegment(
                value: ThemeMode.system,
                icon: const Icon(Icons.brightness_auto, size: 16),
              ),
              ButtonSegment(
                value: ThemeMode.dark,
                icon: const Icon(Icons.dark_mode, size: 16),
              ),
            ],
            selected: {themeMode},
            onSelectionChanged: (modes) =>
                ref.read(themeModeProvider.notifier).set(modes.first),
            style: ButtonStyle(
              visualDensity: VisualDensity.compact,
              tapTargetSize: MaterialTapTargetSize.shrinkWrap,
            ),
          ),
        ),
        _SettingsTile(
          icon: Icons.language,
          title: 'Bahasa',
          trailing: SegmentedButton<Locale>(
            segments: [
              ButtonSegment(
                value: const Locale('id'),
                label: const Text('ID'),
              ),
              ButtonSegment(
                value: const Locale('en'),
                label: const Text('EN'),
              ),
            ],
            selected: {locale},
            onSelectionChanged: (locales) =>
                ref.read(localeProvider.notifier).setLocale(locales.first),
            style: ButtonStyle(
              visualDensity: VisualDensity.compact,
              tapTargetSize: MaterialTapTargetSize.shrinkWrap,
            ),
          ),
        ),

        const SizedBox(height: 8),

        // Account section
        _SectionHeader(title: l10n.account),
        _SettingsTile(
          icon: Icons.lock_outline,
          title: l10n.changePassword,
          onTap: () => context.push('/change-password'),
        ),

        const SizedBox(height: 8),

        // Support section
        _SectionHeader(title: l10n.support),
        _SettingsTile(
          icon: Icons.help_outline,
          title: l10n.faq,
          onTap: () => context.push('/faq'),
        ),
        _SettingsTile(
          icon: Icons.mail_outline,
          title: l10n.contactUs,
          onTap: () => context.push('/contact'),
        ),

        const SizedBox(height: 24),

        // Logout
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: OutlinedButton.icon(
            onPressed: () async {
              final confirmed = await showDialog<bool>(
                context: context,
                builder: (ctx) => AlertDialog(
                  title: Text(l10n.logout),
                  content: const Text('Yakin ingin keluar?'),
                  actions: [
                    TextButton(
                      onPressed: () => Navigator.pop(ctx, false),
                      child: Text(l10n.cancel),
                    ),
                    TextButton(
                      onPressed: () => Navigator.pop(ctx, true),
                      style: TextButton.styleFrom(
                          foregroundColor: isp.danger),
                      child: Text(l10n.logout),
                    ),
                  ],
                ),
              );
              if (confirmed == true) {
                await ref.read(authControllerProvider.notifier).logout();
                if (context.mounted) context.go('/login');
              }
            },
            icon: Icon(Icons.logout, color: isp.danger),
            label: Text(
              l10n.logout,
              style: TextStyle(color: isp.danger),
            ),
            style: OutlinedButton.styleFrom(
              side: BorderSide(color: isp.danger.withOpacity(0.3)),
              minimumSize: const Size.fromHeight(48),
            ),
          ),
        ),
        const SizedBox(height: 32),
      ],
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({required this.title});
  final String title;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 4),
      child: Text(
        title,
        style: TextStyle(
          fontSize: 12,
          fontWeight: FontWeight.w600,
          color: context.isp.textMuted,
          letterSpacing: 0.5,
        ),
      ),
    );
  }
}

class _SettingsTile extends StatelessWidget {
  const _SettingsTile({
    required this.icon,
    required this.title,
    this.trailing,
    this.onTap,
  });

  final IconData icon;
  final String title;
  final Widget? trailing;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return ListTile(
      leading: Icon(icon, color: isp.textMuted),
      title: Text(title),
      trailing: trailing ?? (onTap != null ? const Icon(Icons.chevron_right) : null),
      onTap: onTap,
    );
  }
}
