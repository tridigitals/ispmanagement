import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

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
      appBar: AppBar(
        title: Text(l10n.profile),
      ),
      body: ListView(
        children: [
          const SizedBox(height: IspSpacing.xl),
          // Avatar + Name + Email
          Center(
            child: Column(
              children: [
                CircleAvatar(
                  radius: 48,
                  backgroundColor: isp.accentSurface,
                  backgroundImage: user?.avatarUrl != null
                      ? NetworkImage(_buildAbsoluteUrl(apiBaseUrl, user!.avatarUrl!))
                      : null,
                  child: user?.avatarUrl == null
                      ? Icon(Icons.person, size: 48, color: isp.accent)
                      : null,
                ),
                const SizedBox(height: IspSpacing.md),
                Text(
                  user?.name ?? '—',
                  style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                if (user?.email != null)
                  Text(
                    user!.email,
                    style: TextStyle(color: isp.textMuted),
                  ),
              ],
            ),
          ),
          const SizedBox(height: IspSpacing.xl),
          // Edit Profile
          _ProfileGroup(
            items: [
              _ProfileItem(
                icon: Icons.edit_outlined,
                title: l10n.editProfile,
                onTap: () => GoRouter.of(context).push('/edit-profile'),
              ),
            ],
          ),
          const SizedBox(height: IspSpacing.md),
          // Menu items
          _ProfileGroup(
            items: [
              _ProfileItem(
                icon: Icons.help_outline,
                title: l10n.faq,
                onTap: () => GoRouter.of(context).push('/faq'),
              ),
              _ProfileItem(
                icon: Icons.support_agent_outlined,
                title: l10n.contactUs,
                onTap: () => GoRouter.of(context).push('/contact'),
              ),
              _ProfileItem(
                icon: Icons.notifications_outlined,
                title: l10n.notifications,
                badge: unread > 0 ? '$unread' : null,
                onTap: () => GoRouter.of(context).push('/notifications'),
              ),
              _ProfileItem(
                icon: Icons.lock_outline,
                title: l10n.changePassword,
                onTap: () => GoRouter.of(context).push('/change-password'),
              ),
            ],
          ),
          const SizedBox(height: IspSpacing.md),
          // Theme toggle
          _ThemeToggleCard(
            themeMode: themeMode,
            onChanged: (mode) =>
                ref.read(themeModeProvider.notifier).set(mode),
          ),
          const SizedBox(height: IspSpacing.md),
          // Logout
          _ProfileGroup(
            items: [
              _ProfileItem(
                icon: Icons.logout,
                title: l10n.logout,
                iconColor: isp.danger,
                titleColor: isp.danger,
                onTap: () => ref.read(authControllerProvider.notifier).logout(),
              ),
            ],
          ),
          const SizedBox(height: IspSpacing.xl),
          Center(
            child: Text(
              'v0.1.0+7',
              style: TextStyle(color: isp.textMuted, fontSize: 12),
            ),
          ),
          const SizedBox(height: IspSpacing.xl),
        ],
      ),
    );
  }

  String _buildAbsoluteUrl(String baseUrl, String url) {
    if (url.startsWith('http://') || url.startsWith('https://')) return url;
    return '$baseUrl$url';
  }
}

// ─── Profile Group Card ────────────────────────────────────────

class _ProfileGroup extends StatelessWidget {
  const _ProfileGroup({required this.items});
  final List<Widget> items;
  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return IspCard(
      margin: const EdgeInsets.symmetric(
          horizontal: IspSpacing.lg, vertical: IspSpacing.sm),
      child: Column(
        children: [
          for (var i = 0; i < items.length; i++) ...[
            items[i],
            if (i < items.length - 1)
              Divider(height: 1, color: isp.borderSubtle),
          ],
        ],
      ),
    );
  }
}

// ─── Profile List Item ─────────────────────────────────────────

class _ProfileItem extends StatelessWidget {
  const _ProfileItem({
    required this.icon,
    required this.title,
    required this.onTap,
    this.badge,
    this.iconColor,
    this.titleColor,
  });
  final IconData icon;
  final String title;
  final VoidCallback onTap;
  final String? badge;
  final Color? iconColor;
  final Color? titleColor;
  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return ListTile(
      leading: Icon(icon, color: iconColor),
      title: Text(title, style: TextStyle(color: titleColor)),
      trailing: badge != null
          ? Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
              decoration: BoxDecoration(
                color: isp.danger,
                borderRadius: BorderRadius.circular(IspRadii.pill),
              ),
              child: Text(
                badge!,
                style: const TextStyle(
                  color: Colors.white,
                  fontSize: 11,
                  fontWeight: FontWeight.w700,
                ),
              ),
            )
          : Icon(Icons.chevron_right, color: isp.textMuted),
      onTap: onTap,
    );
  }
}

// ─── Theme Toggle Card ─────────────────────────────────────────

class _ThemeToggleCard extends StatelessWidget {
  const _ThemeToggleCard({
    required this.themeMode,
    required this.onChanged,
  });
  final ThemeMode themeMode;
  final ValueChanged<ThemeMode> onChanged;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return IspCard(
      margin: const EdgeInsets.symmetric(
          horizontal: IspSpacing.lg, vertical: IspSpacing.sm),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        child: Row(
          children: [
            Icon(
              themeMode == ThemeMode.dark
                  ? Icons.dark_mode
                  : Icons.light_mode,
              color: isp.accent,
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                l10n.darkMode,
                style: const TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ),
            // Compact toggle buttons
            _buildToggle(isp),
          ],
        ),
      ),
    );
  }

  Widget _buildToggle(IspThemeColors isp) {
    return Container(
      decoration: BoxDecoration(
        color: isp.surfaceElevated,
        borderRadius: BorderRadius.circular(IspRadii.pill),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          _ToggleBtn(
            icon: Icons.light_mode,
            isSelected: themeMode == ThemeMode.light,
            isp: isp,
            onTap: () => onChanged(ThemeMode.light),
          ),
          _ToggleBtn(
            icon: Icons.brightness_auto,
            isSelected: themeMode == ThemeMode.system,
            isp: isp,
            onTap: () => onChanged(ThemeMode.system),
          ),
          _ToggleBtn(
            icon: Icons.dark_mode,
            isSelected: themeMode == ThemeMode.dark,
            isp: isp,
            onTap: () => onChanged(ThemeMode.dark),
          ),
        ],
      ),
    );
  }
}

class _ToggleBtn extends StatelessWidget {
  const _ToggleBtn({
    required this.icon,
    required this.isSelected,
    required this.isp,
    required this.onTap,
  });
  final IconData icon;
  final bool isSelected;
  final IspThemeColors isp;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        padding: const EdgeInsets.all(8),
        decoration: BoxDecoration(
          color: isSelected ? isp.accent : Colors.transparent,
          shape: BoxShape.circle,
        ),
        child: Icon(
          icon,
          size: 18,
          color: isSelected ? Colors.white : isp.textMuted,
        ),
      ),
    );
  }
}
