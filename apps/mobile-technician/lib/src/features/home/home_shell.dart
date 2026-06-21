import 'package:flutter/material.dart';
import 'package:ui_kit/ui_kit.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../l10n/app_localizations.dart';
import '../../services/notifications_providers.dart';
import '../../services/settings_providers.dart';
import '../../services/auth_providers.dart';
import './home_tab.dart';
import 'tickets_tab.dart';

class HomeShell extends ConsumerStatefulWidget {
  const HomeShell({super.key});
  @override
  ConsumerState<HomeShell> createState() => _State();
}

class _State extends ConsumerState<HomeShell> {
  late final IspThemeColors isp;

  String _normalizeAction(String? actionUrl) {
    if (actionUrl == null || actionUrl.isEmpty) return '/notifications';
    if (actionUrl.startsWith('/support/')) {
      final id = actionUrl.substring('/support/'.length);
      if (id.isNotEmpty) return '/tickets/$id';
    }
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
    final tabStr = GoRouterState.of(context).uri.queryParameters['tab'];
    if (tabStr != null) {
      final tabIdx = int.tryParse(tabStr);
      if (tabIdx != null && tabIdx >= 0 && tabIdx < 2) {
        ref.read(currentTabProvider.notifier).state = tabIdx;
      }
    }
  }

  Future<void> _checkOnboarding() async {}

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final tab = ref.watch(currentTabProvider);
    final user = ref.watch(currentUserProvider);
    final unread = ref.watch(unreadNotificationsCountProvider).valueOrNull ?? 0;

    final tabTitles = [
      '${l10n.hiPrefix}, ${user?.name.split(' ').first ?? ''} 👋',
      l10n.myTickets,
    ];
    final pages = const [
      HomeTab(),
      TicketsTab(),
    ];

    return Scaffold(
      appBar: AppBar(
        title: Text(tabTitles[tab]),
        automaticallyImplyLeading: false,
        actions: [
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
          IconButton(
            icon: const Icon(Icons.account_circle_outlined),
            onPressed: () => GoRouter.of(context).push('/profile'),
          ),
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
            icon: Icons.assignment_outlined,
            selectedIcon: Icons.assignment,
            label: l10n.myTickets,
          ),
        ],
      ),
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
