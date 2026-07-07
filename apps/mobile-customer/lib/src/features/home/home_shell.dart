import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:ui_kit/ui_kit.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../l10n/app_localizations.dart';
import '../../services/notifications_providers.dart';
import '../../services/settings_providers.dart';
import '../../services/auth_providers.dart';
import './home_tab.dart';
import 'invoices_tab.dart';
import 'subscriptions_tab.dart';
import 'support_tab.dart';

class HomeShell extends ConsumerStatefulWidget {
  const HomeShell({super.key});
  @override
  ConsumerState<HomeShell> createState() => _State();
}

class _State extends ConsumerState<HomeShell> {
  DateTime? _lastBackPress;

  /// Normalize action_url → in-app route.
  String _normalizeAction(String? actionUrl) {
    if (actionUrl == null || actionUrl.isEmpty) return '/notifications';
    if (actionUrl.startsWith('/support/')) {
      final id = actionUrl.substring('/support/'.length);
      if (id.isNotEmpty) return '/tickets/$id';
    }
    if (actionUrl.startsWith('/pay/') || actionUrl.startsWith('/invoices'))
      return '/?tab=2';
    if (actionUrl.startsWith('/subscriptions/') ||
        actionUrl.startsWith('/services')) return '/?tab=1';
    if (actionUrl.startsWith('/announcements/')) return '/?tab=0';
    return actionUrl;
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final tabStr = GoRouterState.of(context).uri.queryParameters['tab'];
    if (tabStr != null) {
      final tabIdx = int.tryParse(tabStr);
      if (tabIdx != null && tabIdx >= 0 && tabIdx < 4) {
        // Only switch if different — prevent stale query param from
        // overriding current tab when popping back from detail screens.
        final current = ref.read(currentTabProvider);
        if (tabIdx != current) {
          ref.read(currentTabProvider.notifier).state = tabIdx;
        }
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final tab = ref.watch(currentTabProvider);
    final user = ref.watch(currentUserProvider);
    final unread = ref.watch(unreadNotificationsCountProvider).valueOrNull ?? 0;

    final tabTitles = [
      '${l10n.hiPrefix}, ${user?.name.split(' ').first ?? ''} 👋',
      l10n.mySubscriptions,
      l10n.myInvoices,
      l10n.support,
    ];
    final pages = const [
      HomeTab(),
      SubscriptionsTab(),
      InvoicesTab(),
      SupportTab(),
    ];

    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, result) {
        if (didPop) return;
        if (tab != 0) {
          ref.read(currentTabProvider.notifier).state = 0;
          return;
        }
        final now = DateTime.now();
        if (_lastBackPress == null ||
            now.difference(_lastBackPress!) > const Duration(seconds: 2)) {
          _lastBackPress = now;
          ScaffoldMessenger.of(context)
            ..hideCurrentSnackBar()
            ..showSnackBar(
              SnackBar(
                content: const Text('Tekan sekali lagi untuk keluar'),
                duration: const Duration(seconds: 2),
                behavior: SnackBarBehavior.floating,
              ),
            );
          return;
        }
        SystemNavigator.pop();
      },
      child: Scaffold(
        body: SafeArea(
          child: Stack(
            children: [
              // AppBar via SafeArea-embedded header
              Positioned(
                top: 0,
                left: 0,
                right: 0,
                child: _HomeHeader(
                  title: tabTitles[tab],
                  isp: isp,
                  unread: unread,
                  onNotifications: () =>
                      GoRouter.of(context).push('/notifications'),
                  onAccount: () => GoRouter.of(context).push('/profile'),
                  onSettings: () => GoRouter.of(context).push('/settings'),
                ),
              ),
              // Pages
              Padding(
                padding: const EdgeInsets.only(top: 72),
                child: IndexedStack(index: tab, children: pages),
              ),
              // FAB on support tab
              if (tab == 3)
                Positioned(
                  bottom: 96,
                  right: 20,
                  child: FloatingActionButton(
                    mini: true,
                    backgroundColor: isp.accent,
                    foregroundColor: Colors.white,
                    onPressed: () =>
                        GoRouter.of(context).push('/tickets/new'),
                    child: const Icon(Icons.add),
                  ),
                ),
            ],
          ),
        ),
        bottomNavigationBar: _FloatingGlassNav(
          selectedIndex: tab,
          isp: isp,
          onDestinationSelected: (i) =>
              ref.read(currentTabProvider.notifier).state = i,
          destinations: [
            _NavItem(icon: Icons.home, label: l10n.home),
            _NavItem(icon: Icons.wifi, label: l10n.mySubscriptions),
            _NavItem(icon: Icons.receipt_long, label: l10n.myInvoices),
            _NavItem(icon: Icons.headset_mic, label: l10n.support),
          ],
        ),
      ),
    );
  }
}

// ─── Compact Home Header (replaces AppBar) ──────────────────────

class _HomeHeader extends StatelessWidget {
  const _HomeHeader({
    required this.title,
    required this.isp,
    required this.unread,
    required this.onNotifications,
    required this.onAccount,
    required this.onSettings,
  });

  final String title;
  final IspThemeColors isp;
  final int unread;
  final VoidCallback onNotifications;
  final VoidCallback onAccount;
  final VoidCallback onSettings;

  @override
  Widget build(BuildContext context) {
    // Extract greeting + name from title
    final parts = title.split(', ');
    final greeting = parts.length > 1 ? parts[0] : '';
    final name = parts.length > 1 ? parts.sublist(1).join(', ') : title;

    return Container(
      padding: const EdgeInsets.fromLTRB(20, 8, 16, 12),
      decoration: BoxDecoration(color: isp.background),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  greeting,
                  style: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w700,
                    letterSpacing: 2,
                    color: isp.textMuted,
                    textBaseline: TextBaseline.alphabetic,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  name,
                  style: TextStyle(
                    fontSize: 26,
                    fontWeight: FontWeight.w800,
                    letterSpacing: -.5,
                    color: isp.textPrimary,
                    textBaseline: TextBaseline.alphabetic,
                  ),
                ),
              ],
            ),
          ),
          // Notification bell with badge
          _HeaderIconButton(
            icon: Icons.notifications_outlined,
            isp: isp,
            badgeCount: unread,
            onTap: onNotifications,
          ),
          const SizedBox(width: 8),
          // Avatar — person icon (static, not dynamic)
          _HeaderIconButton(
            icon: Icons.person,
            isp: isp,
            onTap: onAccount,
          ),
        ],
      ),
    );
  }
}

class _HeaderIconButton extends StatelessWidget {
  const _HeaderIconButton({
    required this.icon,
    required this.isp,
    this.badgeCount,
    required this.onTap,
  });

  final IconData icon;
  final IspThemeColors isp;
  final int? badgeCount;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        width: 42,
        height: 42,
        decoration: BoxDecoration(
          color: isp.surface,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: isp.border, width: 1),
        ),
        child: Stack(
          clipBehavior: Clip.none,
          children: [
            Center(
              child: Icon(icon, size: 20, color: isp.textSecondary),
            ),
            if (badgeCount != null && badgeCount! > 0)
              Positioned(
                top: -4,
                right: -4,
                child: Container(
                  width: 18,
                  height: 18,
                  decoration: BoxDecoration(
                    color: isp.danger,
                    shape: BoxShape.circle,
                    border: Border.all(color: isp.background, width: 2),
                  ),
                  child: Center(
                    child: Text(
                      badgeCount! > 9 ? '9+' : '$badgeCount',
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 9,
                        fontWeight: FontWeight.w800,
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

// ─── Nav data ────────────────────────────────────────────────────

class _NavItem {
  const _NavItem({required this.icon, required this.label});
  final IconData icon;
  final String label;
}

// ─── Floating Glass Bottom Nav ──────────────────────────────────

class _FloatingGlassNav extends StatelessWidget {
  const _FloatingGlassNav({
    required this.selectedIndex,
    required this.isp,
    required this.onDestinationSelected,
    required this.destinations,
  });

  final int selectedIndex;
  final IspThemeColors isp;
  final ValueChanged<int> onDestinationSelected;
  final List<_NavItem> destinations;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      top: false,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 0, 20, 12),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(999),
          child: BackdropFilter(
            filter: ui.ImageFilter.blur(sigmaX: 24, sigmaY: 24),
            child: Container(
              decoration: BoxDecoration(
                color: const Color(0xDD111119),
                borderRadius: BorderRadius.circular(999),
                border: Border.all(
                  color: isp.border.withOpacity(0.6),
                  width: 1.5,
                ),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withOpacity(0.4),
                    blurRadius: 20,
                    offset: const Offset(0, 4),
                  ),
                ],
              ),
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 6),
              child: Row(
                children: List.generate(destinations.length, (i) {
                  final selected = i == selectedIndex;
                  final d = destinations[i];
                  return Expanded(
                    child: _GlassNavItem(
                      icon: d.icon,
                      label: d.label,
                      selected: selected,
                      isp: isp,
                      onTap: () => onDestinationSelected(i),
                    ),
                  );
                }),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

// ─── Individual Nav Item (scale animation on tap) ───────────────

class _GlassNavItem extends StatefulWidget {
  const _GlassNavItem({
    required this.icon,
    required this.label,
    required this.selected,
    required this.isp,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final IspThemeColors isp;
  final VoidCallback onTap;

  @override
  State<_GlassNavItem> createState() => _GlassNavItemState();
}

class _GlassNavItemState extends State<_GlassNavItem>
    with SingleTickerProviderStateMixin {
  late final AnimationController _ctrl;
  late final Animation<double> _scale;

  @override
  void initState() {
    super.initState();
    _ctrl = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 150),
    );
    _scale = Tween<double>(begin: 1.0, end: 0.88).animate(
      CurvedAnimation(parent: _ctrl, curve: Curves.easeInOut),
    );
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final isp = widget.isp;
    final selected = widget.selected;

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTapDown: (_) => _ctrl.forward(),
      onTapUp: (_) {
        _ctrl.reverse();
        widget.onTap();
      },
      onTapCancel: () => _ctrl.reverse(),
      child: AnimatedBuilder(
        animation: _scale,
        builder: (_, child) => Transform.scale(
          scale: _scale.value,
          child: child,
        ),
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 250),
          padding: const EdgeInsets.symmetric(vertical: 8),
          decoration: BoxDecoration(
            color: selected ? isp.accent.withOpacity(0.12) : Colors.transparent,
            borderRadius: BorderRadius.circular(999),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                widget.icon,
                size: 22,
                color: selected ? isp.accent : isp.textMuted,
              ),
              const SizedBox(height: 2),
              Text(
                widget.label,
                style: TextStyle(
                  fontSize: 10,
                  fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
                  color: selected ? isp.accentLight : isp.textMuted,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 3),
              // Accent indicator dot
              AnimatedContainer(
                duration: const Duration(milliseconds: 250),
                width: selected ? 5 : 0,
                height: selected ? 5 : 0,
                decoration: BoxDecoration(
                  color: isp.accent,
                  shape: BoxShape.circle,
                  boxShadow: selected
                      ? [
                          BoxShadow(
                            color: isp.accent.withOpacity(0.5),
                            blurRadius: 6,
                          ),
                        ]
                      : null,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
