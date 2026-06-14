import 'package:flutter/material.dart';
import 'package:ui_kit/ui_kit.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:api_client/api_client.dart' show NotificationModel;

import '../../l10n/app_localizations.dart';
import '../../services/notifications_providers.dart';
import '../../theme/app_theme.dart';
import './home_tab.dart';
import 'invoices_tab.dart';
import 'subscriptions_tab.dart';
import 'support_tab.dart';
import '../profile/profile_screen.dart';

class HomeShell extends ConsumerStatefulWidget {
  const HomeShell({super.key});
  @override
  ConsumerState<HomeShell> createState() => _State();
}

class _State extends ConsumerState<HomeShell> {
  late final IspThemeColors isp;
  int _tab = 0;
  Set<String> _knownIds = {};
  bool _notificationsInitialised = false;

  /// Normalize action_url → in-app tab route.
  String _normalizeAction(String? actionUrl) {
    if (actionUrl == null || actionUrl.isEmpty) return '/notifications';
    if (actionUrl.startsWith('/support/')) return '/?tab=3';
    if (actionUrl.startsWith('/pay/') || actionUrl.startsWith('/invoices'))
      return '/?tab=2';
    if (actionUrl.startsWith('/subscriptions/') ||
        actionUrl.startsWith('/services')) return '/?tab=1';
    if (actionUrl.startsWith('/announcements/')) return '/?tab=0';
    return actionUrl;
  }

  void _showNotificationToast(BuildContext context, NotificationModel n) {
    final messenger = ScaffoldMessenger.maybeOf(context);
    if (messenger == null) return;
    messenger.clearSnackBars();
    final route = _normalizeAction(n.actionUrl);
    messenger.showSnackBar(
      SnackBar(
        content: Row(
          children: [
            Container(
              padding: const EdgeInsets.all(6),
              decoration: BoxDecoration(
                color: Colors.white.withOpacity(0.2),
                shape: BoxShape.circle,
              ),
              child: const Icon(
                Icons.notifications_active_rounded,
                color: Colors.white,
                size: 18,
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    n.title,
                    style: const TextStyle(
                      fontWeight: FontWeight.w700,
                      fontSize: 14,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  if (n.body.isNotEmpty)
                    Text(
                      n.body,
                      style: const TextStyle(fontSize: 12),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                ],
              ),
            ),
          ],
        ),
        backgroundColor: isp.accent,
        behavior: SnackBarBehavior.floating,
        margin: const EdgeInsets.fromLTRB(12, 0, 12, 76),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
        ),
        duration: const Duration(seconds: 4),
        action: SnackBarAction(
          label: 'Buka',
          textColor: Colors.white,
          onPressed: () {
            try {
              if (route == '/notifications') {
                context.push(route);
              } else {
                context.go(route);
              }
            } catch (_) {}
            // Mark this notif as read.
            ref.read(notificationsProvider.notifier).markRead(n.id);
          },
        ),
      ),
    );
  }

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _checkOnboarding());
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // Always sync tab from URL query param so go('/?tab=N') works at any time.
    final tabStr = GoRouterState.of(context).uri.queryParameters['tab'];
    if (tabStr != null) {
      final tabIdx = int.tryParse(tabStr);
      if (tabIdx != null && tabIdx >= 0 && tabIdx < 5 && tabIdx != _tab) {
        setState(() => _tab = tabIdx);
      }
    }
  }

  Future<void> _checkOnboarding() async {
    // Biometric auth is handled in LoginScreen — no duplicate prompt here.
  }

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
final l10n = AppLocalizations.of(context);
    final notifState = ref.watch(notificationsProvider);

    // Detect new notifications arriving via polling.
    final currentIds =
        notifState.valueOrNull?.map((n) => n.id).toSet() ?? <String>{};
    if (notifState.hasValue) {
      if (!_notificationsInitialised) {
        _knownIds = currentIds;
        _notificationsInitialised = true;
      } else {
        final newIds = currentIds.difference(_knownIds);
        if (newIds.isNotEmpty) {
          final notifs = notifState.valueOrNull!;
          // Show toast for the most recent new one.
          final newNotif = notifs
              .where((n) => newIds.contains(n.id) && n.readAt == null)
              .toList()
            ..sort((a, b) => b.createdAt.compareTo(a.createdAt));
          if (newNotif.isNotEmpty) {
            WidgetsBinding.instance.addPostFrameCallback((_) {
              if (mounted) _showNotificationToast(context, newNotif.first);
            });
          }
          _knownIds = currentIds;
        }
      }
    }

    final pages = const [
      HomeTab(),
      SubscriptionsTab(),
      InvoicesTab(),
      SupportTab(),
      ProfileScreen(),
    ];
    return Scaffold(
      body: IndexedStack(index: _tab, children: pages),
      bottomNavigationBar: _CleanNavBar(
        selectedIndex: _tab,
        onDestinationSelected: (i) => setState(() => _tab = i),
        destinations: [
          _NavDestination(
            icon: Icons.home_outlined,
            selectedIcon: Icons.home,
            label: l10n.home,
          ),
          _NavDestination(
            icon: Icons.wifi_outlined,
            selectedIcon: Icons.wifi,
            label: l10n.mySubscriptions,
          ),
          _NavDestination(
            icon: Icons.receipt_long_outlined,
            selectedIcon: Icons.receipt_long,
            label: l10n.myInvoices,
          ),
          _NavDestination(
            icon: Icons.headset_mic_outlined,
            selectedIcon: Icons.headset_mic,
            label: l10n.support,
          ),
          _NavDestination(
            icon: Icons.person_outline,
            selectedIcon: Icons.person,
            label: l10n.profile,
          ),
        ],
      ),
    );
  }
}

// ── Nav destination data ──────────────────────────────────────────

class _NavDestination {
  const _NavDestination({
    required this.icon,
    required this.selectedIcon,
    required this.label,
  });
  final IconData icon;
  final IconData selectedIcon;
  final String label;
}

// ── Clean flat bottom nav bar (Revolut + Linear style) ────────────

class _CleanNavBar extends StatelessWidget {
  const _CleanNavBar({
    required this.selectedIndex,
    required this.onDestinationSelected,
    required this.destinations,
  });

  final int selectedIndex;
  final ValueChanged<int> onDestinationSelected;
  final List<_NavDestination> destinations;

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return Container(
      height: 64,
      decoration: BoxDecoration(
        color: isp.surface,
        border: Border(
          top: BorderSide(color: isp.border, width: 1),
        ),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceEvenly,
        children: List.generate(destinations.length, (index) {
          final dest = destinations[index];
          final isSelected = index == selectedIndex;
          return _NavBarItem(
            destination: dest,
            isSelected: isSelected,
            onTap: () => onDestinationSelected(index),
          );
        }),
      ),
    );
  }
}

class _NavBarItem extends StatelessWidget {
  const _NavBarItem({
    required this.destination,
    required this.isSelected,
    required this.onTap,
  });

  final _NavDestination destination;
  final bool isSelected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return GestureDetector(
      onTap: onTap,
      behavior: HitTestBehavior.opaque,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 250),
        curve: Curves.easeOutCubic,
        padding: isSelected
            ? const EdgeInsets.symmetric(horizontal: 16, vertical: 8)
            : const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: isSelected
            ? BoxDecoration(
                color: isp.accent,
                borderRadius: BorderRadius.circular(9999),
              )
            : null,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              isSelected ? destination.selectedIcon : destination.icon,
              color: isSelected ? Colors.white : isp.textMuted,
              size: isSelected ? 20 : 24,
            ),
            if (isSelected) ...[
              const SizedBox(width: 6),
              Text(
                destination.label,
                style: const TextStyle(
                  color: Colors.white,
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
