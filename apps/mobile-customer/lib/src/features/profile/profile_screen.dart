import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../services/notifications_providers.dart';

class ProfileScreen extends ConsumerWidget {
  const ProfileScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
final l10n = AppLocalizations.of(context);
    final user = ref.watch(currentUserProvider);
    final unread = ref.watch(unreadNotificationsCountProvider).valueOrNull ?? 0;
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.profile),
        actions: [
          IconButton(
            icon: const Icon(Icons.settings),
            onPressed: () => GoRouter.of(context).push('/settings'),
          ),
        ],
      ),
      body: ListView(
        children: [
          const SizedBox(height: IspSpacing.xl),
          Center(
            child: Column(
              children: [
                CircleAvatar(
                  radius: 48,
                  backgroundColor: isp.accentSurface,
                  backgroundImage: user?.avatarUrl != null
                      ? NetworkImage(user!.avatarUrl!)
                      : null,
                  child: user?.avatarUrl == null
                      ? Icon(Icons.person,
                          size: 48, color: isp.accent)
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
              'v0.1.0+1',
              style: TextStyle(color: isp.textMuted, fontSize: 12),
            ),
          ),
          const SizedBox(height: IspSpacing.xl),
        ],
      ),
    );
  }
}

class _ProfileGroup extends StatelessWidget {
  const _ProfileGroup({required this.items});
  final List<_ProfileItem> items;
  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return IspCard(
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


    final isp = context.isp;    return ListTile(
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
