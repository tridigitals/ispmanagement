import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../services/feature_providers.dart';
import '../home/home_tab.dart';
import '../invoices/invoices_tab.dart';
import '../subscriptions/subscriptions_tab.dart';
import '../support/support_tab.dart';
import '../profile/profile_screen.dart';

class HomeShell extends ConsumerStatefulWidget {
  const HomeShell({super.key});
  @override
  ConsumerState<HomeShell> createState() => _State();
}

class _State extends ConsumerState<HomeShell> {
  int _tab = 0;
  bool _gateChecked = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _checkOnboarding());
  }

  Future<void> _checkOnboarding() async {
    if (_gateChecked) return;
    _gateChecked = true;
    final ok = ref.read(biometricEnabledProvider).valueOrNull;
    if (ok == true) {
      final res = await ref
          .read(authControllerProvider.notifier)
          .tryBiometricUnlock();
      res.fold(
        (_) {
          // success, stay
        },
        (_) {
          // failed — go to login
          if (mounted) context.go('/login');
        },
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final pages = const [
      HomeTab(),
      SubscriptionsTab(),
      InvoicesTab(),
      SupportTab(),
      ProfileScreen(),
    ];
    return Scaffold(
      body: IndexedStack(index: _tab, children: pages),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _tab,
        onDestinationSelected: (i) => setState(() => _tab = i),
        destinations: [
          NavigationDestination(
            icon: const Icon(Icons.home_outlined),
            selectedIcon: const Icon(Icons.home),
            label: l10n.home,
          ),
          NavigationDestination(
            icon: const Icon(Icons.wifi_outlined),
            selectedIcon: const Icon(Icons.wifi),
            label: l10n.mySubscriptions,
          ),
          NavigationDestination(
            icon: const Icon(Icons.receipt_long_outlined),
            selectedIcon: const Icon(Icons.receipt_long),
            label: l10n.myInvoices,
          ),
          NavigationDestination(
            icon: const Icon(Icons.headset_mic_outlined),
            selectedIcon: const Icon(Icons.headset_mic),
            label: l10n.support,
          ),
          NavigationDestination(
            icon: const Icon(Icons.person_outline),
            selectedIcon: const Icon(Icons.person),
            label: l10n.profile,
          ),
        ],
      ),
    );
  }
}
