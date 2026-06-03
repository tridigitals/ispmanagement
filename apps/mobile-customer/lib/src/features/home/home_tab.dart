import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../services/customer_data_providers.dart';
import '../../services/notifications_providers.dart';

class HomeTab extends ConsumerWidget {
  const HomeTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context)!;
    final user = ref.watch(currentUserProvider).valueOrNull;
    final subState = ref.watch(mySubscriptionsProvider);
    final invState = ref.watch(myInvoicesProvider);
    final unread = ref.watch(unreadNotificationsCountProvider).valueOrNull ?? 0;

    return RefreshIndicator(
      onRefresh: () async {
        ref.invalidate(mySubscriptionsProvider);
        ref.invalidate(myInvoicesProvider);
        ref.invalidate(unreadNotificationsCountProvider);
        await Future.wait([
          ref.read(mySubscriptionsProvider.future),
          ref.read(myInvoicesProvider.future),
        ]);
      },
      child: CustomScrollView(
        slivers: [
          SliverAppBar(
            pinned: true,
            title: Text(
              '${l10n.hiPrefix}, ${user?.name.split(' ').first ?? ''} 👋',
            ),
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
                          decoration: const BoxDecoration(
                            color: IspColors.danger,
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
                onPressed: () => context.push('/notifications'),
              ),
              IconButton(
                icon: const Icon(Icons.account_circle_outlined),
                onPressed: () => context.push('/profile'),
              ),
            ],
          ),
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(
              IspSpacing.lg,
              IspSpacing.md,
              IspSpacing.lg,
              IspSpacing.xxl,
            ),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                _PrimarySubscription(subState: subState),
                const SizedBox(height: IspSpacing.lg),
                _StatsRow(subState: subState, invState: invState),
                const SizedBox(height: IspSpacing.lg),
                _QuickActions(),
                const SizedBox(height: IspSpacing.lg),
                _RecentInvoices(invState: invState),
              ]),
            ),
          ),
        ],
      ),
    );
  }
}

class _PrimarySubscription extends ConsumerWidget {
  const _PrimarySubscription({required this.subState});
  final AsyncValue<PaginatedResponse<SubscriptionModel>> subState;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context)!;
    return subState.when(
      loading: () => const _Skeleton(height: 180),
      error: (e, _) => _ErrorCard(message: e.toString()),
      data: (page) {
        if (page.data.isEmpty) return _EmptyState(label: l10n.noSubscription);
        final active = page.data.firstWhere(
          (s) => s.isActive,
          orElse: () => page.data.first,
        );
        return _SubscriptionHeroCard(sub: active);
      },
    );
  }
}

class _SubscriptionHeroCard extends StatelessWidget {
  const _SubscriptionHeroCard({required this.sub});
  final SubscriptionModel sub;

  @override
  Widget build(BuildContext context) {
    final fmt = NumberFormat.simpleCurrency(name: sub.currencyCode);
    return Container(
      padding: const EdgeInsets.all(IspSpacing.xl),
      decoration: BoxDecoration(
        gradient: const LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [IspColors.primary, Color(0xFF6677EE)],
        ),
        borderRadius: BorderRadius.circular(IspRadii.xl),
        boxShadow: IspShadows.md,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                sub.packageName ?? 'Paket Internet',
                style: const TextStyle(color: Colors.white70, fontSize: 13),
              ),
              IspStatusBadge(
                label: sub.statusLabel(),
                tone: sub.isActive
                    ? StatusTone.success
                    : sub.needsAttention
                        ? StatusTone.danger
                        : StatusTone.warning,
              ),
            ],
          ),
          const SizedBox(height: 24),
          Text(
            fmt.format(sub.price),
            style: const TextStyle(
              color: Colors.white,
              fontSize: 36,
              fontWeight: FontWeight.w800,
            ),
          ),
          Text(
            '/ ${sub.billingCycle}',
            style: const TextStyle(color: Colors.white70, fontSize: 13),
          ),
          const SizedBox(height: 24),
          Row(
            children: [
              const Icon(Icons.router, color: Colors.white70, size: 16),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  sub.routerName ?? sub.locationLabel ?? '-',
                  style: const TextStyle(color: Colors.white, fontSize: 13),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _StatsRow extends StatelessWidget {
  const _StatsRow({required this.subState, required this.invState});
  final AsyncValue<PaginatedResponse<SubscriptionModel>> subState;
  final AsyncValue<PaginatedResponse<InvoiceModel>> invState;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(
          child: IspStatCard(
            label: 'Tagihan Belum Bayar',
            value: invState.maybeWhen(
              data: (page) => page.data.where((i) => i.isUnpaid).length.toString(),
              orElse: () => '–',
            ),
            helper: invState.maybeWhen(
              data: (page) {
                final unpaid = page.data.where((i) => i.isUnpaid).toList();
                if (unpaid.isEmpty) return 'Tidak ada tagihan';
                return unpaid.first.invoiceNumber;
              },
              orElse: () => '',
            ),
            icon: Icons.receipt_long,
            tone: StatusTone.warning,
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: IspStatCard(
            label: 'Paket Aktif',
            value: subState.maybeWhen(
              data: (page) => page.data.where((s) => s.isActive).length.toString(),
              orElse: () => '–',
            ),
            helper: 'Dari total langganan',
            icon: Icons.wifi,
            tone: StatusTone.success,
          ),
        ),
      ],
    );
  }
}

class _QuickActions extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: IspSpacing.md),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceAround,
          children: [
            _QuickAction(
              icon: Icons.speed,
              label: 'Speed Test',
              onTap: () {},
            ),
            _QuickAction(
              icon: Icons.payment,
              label: 'Bayar',
              onTap: () {},
            ),
            _QuickAction(
              icon: Icons.headset_mic,
              label: 'Lapor',
              onTap: () => GoRouterWrapper.push(context, '/tickets/new'),
            ),
            _QuickAction(
              icon: Icons.share,
              label: 'Share',
              onTap: () {},
            ),
          ],
        ),
      ),
    );
  }
}

/// Tiny shim to keep _QuickActions free of Riverpod/G router imports
/// (used to keep the build small and predictable).
class GoRouterWrapper {
  GoRouterWrapper._();
  static void push(BuildContext context, String path) {
    // Use the global GoRouter context helper.
    // We import go_router at the top of the file as `context.push(path)`.
    // This wrapper exists purely to allow static-analysis friendly access
    // and will simply delegate to the same call.
    // ignore: avoid_returning_null
    context.push(path);
  }
}

class _QuickAction extends StatelessWidget {
  const _QuickAction({
    required this.icon,
    required this.label,
    required this.onTap,
  });
  final IconData icon;
  final String label;
  final VoidCallback onTap;
  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(IspRadii.md),
      child: Padding(
        padding: const EdgeInsets.all(IspSpacing.md),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: IspColors.primarySubtle,
                borderRadius: BorderRadius.circular(IspRadii.md),
              ),
              child: Icon(icon, color: IspColors.primary, size: 22),
            ),
            const SizedBox(height: 6),
            Text(label, style: const TextStyle(fontSize: 12)),
          ],
        ),
      ),
    );
  }
}

class _RecentInvoices extends StatelessWidget {
  const _RecentInvoices({required this.invState});
  final AsyncValue<PaginatedResponse<InvoiceModel>> invState;
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                l10n.recentInvoices,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              TextButton(
                onPressed: () {},
                child: Text(l10n.seeAll),
              ),
            ],
          ),
        ),
        const SizedBox(height: 8),
        Card(
          child: invState.when(
            loading: () => const _Skeleton(height: 200),
            error: (e, _) => _ErrorCard(message: e.toString()),
            data: (page) {
              if (page.data.isEmpty) {
                return Padding(
                  padding: const EdgeInsets.all(24),
                  child: Center(child: Text(l10n.noInvoices)),
                );
              }
              return Column(
                children: page.data.take(5).map((inv) {
                  return IspListItem(
                    title: inv.invoiceNumber,
                    subtitle: inv.subscriptionLabel ?? inv.notes ?? '',
                    leading: Container(
                      padding: const EdgeInsets.all(8),
                      decoration: BoxDecoration(
                        color: IspColors.bgTertiary,
                        borderRadius: BorderRadius.circular(IspRadii.sm),
                      ),
                      child: const Icon(Icons.receipt_outlined, size: 18),
                    ),
                    trailing: Column(
                      crossAxisAlignment: CrossAxisAlignment.end,
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          fmt.format(inv.amount),
                          style: const TextStyle(fontWeight: FontWeight.w600),
                        ),
                        const SizedBox(height: 2),
                        IspStatusBadge(
                          label: inv.statusLabel(),
                          tone: inv.isPaid
                              ? StatusTone.success
                              : inv.isOverdue
                                  ? StatusTone.danger
                                  : StatusTone.warning,
                        ),
                      ],
                    ),
                    onTap: () => GoRouterWrapper.push(
                      context,
                      '/invoices/${inv.id}',
                    ),
                  );
                }).toList(),
              );
            },
          ),
        ),
      ],
    );
  }
}

class _Skeleton extends StatelessWidget {
  const _Skeleton({required this.height});
  final double height;
  @override
  Widget build(BuildContext context) {
    return Container(
      height: height,
      decoration: BoxDecoration(
        color: IspColors.bgSurface,
        borderRadius: BorderRadius.circular(IspRadii.lg),
      ),
      child: const Center(child: CircularProgressIndicator(strokeWidth: 2)),
    );
  }
}

class _ErrorCard extends StatelessWidget {
  const _ErrorCard({required this.message});
  final String message;
  @override
  Widget build(BuildContext context) {
    return Card(
      color: IspColors.danger.withValues(alpha: 0.1),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            const Icon(Icons.error_outline, color: IspColors.danger),
            const SizedBox(width: 12),
            Expanded(child: Text(message, style: const TextStyle(fontSize: 13))),
          ],
        ),
      ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.label});
  final String label;
  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(
                Icons.inbox_outlined,
                size: 48,
                color: IspColors.textTertiary,
              ),
              const SizedBox(height: 12),
              Text(label, style: const TextStyle(color: IspColors.textTertiary)),
            ],
          ),
        ),
      ),
    );
  }
}
