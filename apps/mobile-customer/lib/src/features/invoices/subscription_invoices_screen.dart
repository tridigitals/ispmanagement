import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';
import 'package:collection/collection.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../services/missing_providers.dart';

BoxDecoration _nbCard(IspThemeColors isp) => BoxDecoration(
      color: isp.surface,
      borderRadius: BorderRadius.circular(16),
      border: Border.all(color: isp.border, width: 1.5),
      boxShadow: [
        BoxShadow(
          color: isp.border.withOpacity(0.5),
          offset: const Offset(3, 3),
          blurRadius: 0,
        ),
      ],
    );

class SubscriptionInvoicesScreen extends ConsumerWidget {
  const SubscriptionInvoicesScreen({required this.subscriptionId, super.key});
  final String subscriptionId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');
    final monthFmt = DateFormat('MMMM', 'id_ID');
    final invAsync = ref.watch(subscriptionInvoicesProvider(subscriptionId));

    return Scaffold(
      appBar: AppBar(title: const Text('Riwayat Tagihan')),
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
                padding: const EdgeInsets.all(24),
                child: Column(mainAxisSize: MainAxisSize.min, children: [
                  Icon(Icons.receipt_long_outlined,
                      size: 64, color: isp.textMuted),
                  const SizedBox(height: 12),
                  Text('Belum ada tagihan',
                      style: TextStyle(color: isp.textMuted, fontSize: 16)),
                ]),
              ),
            );
          }

          // Group by month
          final grouped = groupBy(invoices, (InvoiceModel inv) {
            final d = inv.dueDate ?? DateTime.now();
            return monthFmt.format(d).toUpperCase();
          });

          return RefreshIndicator(
            onRefresh: () async {
              ref.invalidate(subscriptionInvoicesProvider(subscriptionId));
              await ref
                  .read(subscriptionInvoicesProvider(subscriptionId).future);
            },
            child: ListView(
              padding: const EdgeInsets.fromLTRB(16, 16, 16, 100),
              children: grouped.entries.expand((entry) {
                return [
                  // Month header
                  Padding(
                    padding: const EdgeInsets.fromLTRB(4, 16, 0, 8),
                    child: Text(
                      entry.key,
                      style: TextStyle(
                          fontSize: 13,
                          fontWeight: FontWeight.w700,
                          color: isp.textMuted,
                          letterSpacing: 1),
                    ),
                  ),
                  // Invoices in this month
                  ...entry.value.map((inv) =>
                      _InvoiceRow(inv: inv, fmt: fmt, dateFmt: dateFmt)),
                ];
              }).toList(),
            ),
          );
        },
      ),
    );
  }
}

class _InvoiceRow extends StatelessWidget {
  const _InvoiceRow(
      {required this.inv, required this.fmt, required this.dateFmt});
  final InvoiceModel inv;
  final NumberFormat fmt;
  final DateFormat dateFmt;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final isPaid = inv.isPaid;

    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () => GoRouter.of(context).push('/invoices/${inv.id}'),
          borderRadius: BorderRadius.circular(16),
          child: Container(
            decoration: _nbCard(isp),
            clipBehavior: Clip.antiAlias,
            child: IntrinsicHeight(
              child: Row(children: [
                Container(
                    width: 4,
                    color: isPaid
                        ? isp.success
                        : inv.isOverdue
                            ? isp.danger
                            : isp.warning),
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 14, vertical: 12),
                    child: Row(children: [
                      Expanded(
                        child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(inv.invoiceNumber ?? '',
                                  style: TextStyle(
                                      fontSize: 13,
                                      fontWeight: FontWeight.w600,
                                      color: isp.textPrimary)),
                              const SizedBox(height: 2),
                              Text(
                                  dateFmt.format(inv.dueDate ?? DateTime.now()),
                                  style: TextStyle(
                                      fontSize: 11, color: isp.textMuted)),
                            ]),
                      ),
                      Column(
                          crossAxisAlignment: CrossAxisAlignment.end,
                          children: [
                            Text(fmt.format(inv.amount),
                                style: TextStyle(
                                    fontSize: 14,
                                    fontWeight: FontWeight.w700,
                                    color: isp.textPrimary)),
                            const SizedBox(height: 4),
                            Container(
                              padding: const EdgeInsets.symmetric(
                                  horizontal: 8, vertical: 3),
                              decoration: BoxDecoration(
                                color: isPaid
                                    ? isp.success.withOpacity(0.12)
                                    : isp.warning.withOpacity(0.12),
                                borderRadius: BorderRadius.circular(6),
                              ),
                              child: Text(
                                isPaid ? 'Lunas' : 'Jatuh Tempo',
                                style: TextStyle(
                                    fontSize: 10,
                                    fontWeight: FontWeight.w700,
                                    color: isPaid ? isp.success : isp.warning),
                              ),
                            ),
                          ]),
                    ]),
                  ),
                ),
              ]),
            ),
          ),
        ),
      ),
    );
  }
}
