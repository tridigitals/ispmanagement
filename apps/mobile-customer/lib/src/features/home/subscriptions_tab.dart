import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';

class SubscriptionsTab extends ConsumerWidget {
  const SubscriptionsTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context)!;
    final state = ref.watch(mySubscriptionsProvider);
    return CustomScrollView(
      slivers: [
        SliverAppBar(title: Text(l10n.mySubscriptions), pinned: true),
        state.when(
          loading: () => const SliverFillRemaining(
            hasScrollBody: false,
            child: Center(child: CircularProgressIndicator()),
          ),
          error: (e, _) => SliverFillRemaining(
            hasScrollBody: false,
            child: Center(child: Text(e.toString())),
          ),
          data: (page) {
            if (page.data.isEmpty) {
              return const SliverFillRemaining(
                hasScrollBody: false,
                child: Center(child: Text('Belum ada langganan')),
              );
            }
            return SliverList.separated(
              itemBuilder: (_, i) => _SubscriptionTile(sub: page.data[i]),
              separatorBuilder: (_, __) => const SizedBox(height: 8),
              itemCount: page.data.length,
            );
          },
        ),
      ],
    );
  }
}

class _SubscriptionTile extends StatelessWidget {
  const _SubscriptionTile({required this.sub});
  final SubscriptionModel sub;

  @override
  Widget build(BuildContext context) {
    final fmt = NumberFormat.simpleCurrency(name: sub.currencyCode);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: IspSpacing.lg),
      child: Card(
        child: InkWell(
          borderRadius: BorderRadius.circular(IspRadii.lg),
          onTap: () => Navigator.pushNamed(context, '/subscriptions/${sub.id}'),
          child: Padding(
            padding: const EdgeInsets.all(IspSpacing.lg),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Expanded(
                      child: Text(
                        sub.packageName ?? 'Paket',
                        style: const TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.w600,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
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
                const SizedBox(height: 12),
                Row(
                  children: [
                    const Icon(Icons.router, size: 16, color: IspColors.textTertiary),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        sub.routerName ?? sub.locationLabel ?? '-',
                        style: const TextStyle(
                          fontSize: 13,
                          color: IspColors.textTertiary,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                ),
                const Divider(height: 24),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      fmt.format(sub.price),
                      style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
                    ),
                    Text(
                      '/ ${sub.billingCycle}',
                      style: const TextStyle(fontSize: 12, color: IspColors.textTertiary),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
