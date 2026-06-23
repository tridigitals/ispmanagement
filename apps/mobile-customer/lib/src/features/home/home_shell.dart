import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:ui_kit/ui_kit.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:api_client/api_client.dart' show NotificationModel;

import '../../l10n/app_localizations.dart';
import '../../services/notifications_providers.dart';
import '../../services/settings_providers.dart';
import '../../services/auth_providers.dart';
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
  DateTime? _lastBackPress;

  /// Normalize action_url → in-app route.
  String _normalizeAction(String? actionUrl) {
    if (actionUrl == null || actionUrl.isEmpty) return '/notifications';
    // /support/{id} → /tickets/{id} (direct to ticket detail)
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
      if (tabIdx != null && tabIdx >= 0 && tabIdx < 4) {
        ref.read(currentTabProvider.notifier).state = tabIdx;
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
    final tab = ref.watch(currentTabProvider);
    final user = ref.watch(currentUserProvider);
    final unread = ref.watch(unreadNotificationsCountProvider).valueOrNull ?? 0;

    // Tab titles matching bottom nav labels
    final tabTitles = [
      '${l10n.hiPrefix}, ${user?.name.split(' ').first ?? ''} 👋',
      l10n.mySubscriptions,
      l10n.myInvoices,
      l10n.support,
      l10n.profile,
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
          // Switch to Home tab instead of exiting
          ref.read(currentTabProvider.notifier).state = 0;
          return;
        }
        // Already on Home tab — double back to exit
        final now = DateTime.now();
        if (_lastBackPress == null ||
            now.difference(_lastBackPress!) > const Duration(seconds: 2)) {
          _lastBackPress = now;
          ScaffoldMessenger.of(context)
            ..hideCurrentSnackBar()
            ..showSnackBar(
              SnackBar(
                content: const Text('Press back again to exit'),
                duration: const Duration(seconds: 2),
                behavior: SnackBarBehavior.floating,
              ),
            );
          return;
        }
        // Second back press — exit app
        SystemNavigator.pop();
      },
      child: Scaffold(
      appBar: AppBar(
        // Title: greeting on home tab, tab label on others
        title: Text(tabTitles[tab]),
        automaticallyImplyLeading: false,
        actions: [
          // Bell / notifications
          IconButton(
            icon: Stack(
              clipBehavior: Clip.none,
              children: [
                const Icon(Icons.notifications_outlined),
                if (unread > 0)
                  Positioned(
                    top: -2,
                    right: -2,
                    child: Container(
                      padding: const EdgeInsets.all(3),
                      decoration: BoxDecoration(
                        color: isp.danger,
                        shape: BoxShape.circle,
                      ),
                      constraints: const BoxConstraints(
                        minWidth: 14,
                        minHeight: 14,
                      ),
                      child: Text(
                        unread > 9 ? '9+' : '$unread',
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 9,
                          fontWeight: FontWeight.w700,
                        ),
                        textAlign: TextAlign.center,
                      ),
                    ),
                  ),
              ],
            ),
            onPressed: () => GoRouter.of(context).push('/notifications'),
          ),
          // Account
          IconButton(
            icon: const Icon(Icons.account_circle_outlined),
            onPressed: () => GoRouter.of(context).push('/profile'),
          ),
          // Settings
          IconButton(
            icon: const Icon(Icons.settings_outlined),
            tooltip: l10n.settings,
            onPressed: () => GoRouter.of(context).push('/settings'),
          ),
        ],
      ),
      body: SafeArea(
        child: IndexedStack(index: tab, children: pages),
      ),
      floatingActionButton: tab == 3
          ? FloatingActionButton(
              mini: true,
              backgroundColor: isp.accent,
              foregroundColor: Colors.white,
              onPressed: () => GoRouter.of(context).push('/tickets/new'),
              child: const Icon(Icons.add),
            )
          : null,
      bottomNavigationBar: _CleanNavBar(
        selectedIndex: tab,
        onDestinationSelected: (i) =>
            ref.read(currentTabProvider.notifier).state = i,
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
        ],
      ),
      ), // PopScope
    );
  }
}

// ─── Clean NavBar ──────────────────────────────────────────────

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
      decoration: BoxDecoration(
        color: isp.surface,
        border: Border(top: BorderSide(color: isp.borderSubtle)),
      ),
      child: SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
          child: Row(
            children: List.generate(destinations.length, (i) {
              final d = destinations[i];
              final selected = i == selectedIndex;
              return Expanded(
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTap: () => onDestinationSelected(i),
                  child: AnimatedContainer(
                    duration: const Duration(milliseconds: 200),
                    padding: const EdgeInsets.symmetric(vertical: 6),
                    decoration: BoxDecoration(
                      color: selected
                          ? isp.accent.withOpacity(0.12)
                          : Colors.transparent,
                      borderRadius: BorderRadius.circular(IspRadii.pill),
                    ),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          selected ? d.selectedIcon : d.icon,
                          size: 22,
                          color: selected ? isp.accent : isp.textMuted,
                        ),
                        const SizedBox(height: 2),
                        Text(
                          d.label,
                          style: TextStyle(
                            fontSize: 10,
                            fontWeight:
                                selected ? FontWeight.w600 : FontWeight.w400,
                            color: selected ? isp.accent : isp.textMuted,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ],
                    ),
                  ),
                ),
              );
            }),
          ),
        ),
      ),
    );
  }
}

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
