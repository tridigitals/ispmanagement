import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../services/missing_providers.dart';

/// Lists all invoices related to a specific subscription.
class SubscriptionInvoicesScreen extends ConsumerWidget {
  const SubscriptionInvoicesScreen({required this.subscriptionId, super.key});
  final String subscriptionId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');
    final invAsync = ref.watch(subscriptionInvoicesProvider(subscriptionId));

    return Scaffold(
      appBar: AppBar(title: const Text('Tagihan Langganan')),
      body: invAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => IspErrorState(
          message: e.toString(),
          onRetry: () =>
              ref.invalidate(subscriptionInvoicesProvider(subscriptionId)),
        ),
        data: (invoices) {
          if (invoices.isEmpty) {
            return Center(
              child: Padding(
                padding: const EdgeInsets.all(IspSpacing.xl),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.receipt_long_outlined,
                        size: 64, color: isp.textMuted),
                    const SizedBox(height: IspSpacing.md),
                    Text('Belum ada tagihan',
                        style: TextStyle(color: isp.textMuted, fontSize: 16)),
                  ],
                ),
              ),
            );
          }
          return RefreshIndicator(
            onRefresh: () async {
              ref.invalidate(subscriptionInvoicesProvider(subscriptionId));
              await ref.read(subscriptionInvoicesProvider(subscriptionId).future);
            },
            child: ListView.separated(
              padding: const EdgeInsets.all(IspSpacing.lg),
              itemCount: invoices.length,
              separatorBuilder: (_, __) =>
                  const SizedBox(height: IspSpacing.sm),
              itemBuilder: (_, i) {
                final inv = invoices[i];
                return _InvoiceCard(
                  inv: inv,
                  fmt: fmt,
                  dateFmt: dateFmt,
                  onTap: () => GoRouter.of(context).push('/invoices/${inv.id}'),
                );
              },
            ),
          );
        },
      ),
    );
  }
}

class _InvoiceCard extends StatelessWidget {
  const _InvoiceCard({
    required this.inv,
    required this.fmt,
    required this.dateFmt,
    required this.onTap,
  });
  final InvoiceModel inv;
  final NumberFormat fmt;
  final DateFormat dateFmt;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final isPaid = inv.isPaid;

    return Container(
      decoration: NbStyle.card(context, radius: BorderRadius.circular(IspRadii.lg)),
      child: Material(
        color: Colors.transparent,
        borderRadius: BorderRadius.circular(IspRadii.lg),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(IspRadii.lg),
          child: Padding(
            padding: const EdgeInsets.all(IspSpacing.md),
            child: Row(
              children: [
                // Status indicator
                Container(
                  width: 4,
                  height: 48,
                  decoration: BoxDecoration(
                    color: isPaid
                        ? IspThemeColors.of(context).success
                        : inv.isOverdue
                            ? IspThemeColors.of(context).danger
                            : IspThemeColors.of(context).warning,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
                const SizedBox(width: IspSpacing.md),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        inv.invoiceNumber,
                        style: const TextStyle(
                          fontWeight: FontWeight.w600,
                          fontSize: 15,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        'Jatuh tempo: ${dateFmt.format(inv.dueDate)}',
                        style: TextStyle(
                          color: isp.textMuted,
                          fontSize: 12,
                        ),
                      ),
                    ],
                  ),
                ),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: [
                    Text(
                      fmt.format(inv.amount),
                      style: TextStyle(
                        fontWeight: FontWeight.w700,
                        fontSize: 15,
                        color: isPaid ? IspThemeColors.of(context).success : isp.textPrimary,
                      ),
                    ),
                    IspStatusBadge(
                      label: inv.statusLabel(),
                      tone: isPaid
                          ? StatusTone.success
                          : inv.isOverdue
                              ? StatusTone.danger
                              : StatusTone.warning,
                    ),
                  ],
                ),
                const SizedBox(width: IspSpacing.sm),
                Icon(Icons.chevron_right, color: isp.textMuted),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
