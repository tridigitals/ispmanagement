import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';

class InvoicesTab extends ConsumerWidget {
  const InvoicesTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context)!;
    final state = ref.watch(myInvoicesProvider);
    return CustomScrollView(
      slivers: [
        SliverAppBar(title: Text(l10n.myInvoices), pinned: true),
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
                child: Center(child: Text('Belum ada tagihan')),
              );
            }
            return SliverList.separated(
              itemBuilder: (_, i) => _InvoiceTile(inv: page.data[i]),
              separatorBuilder: (_, __) => const SizedBox(height: 8),
              itemCount: page.data.length,
            );
          },
        ),
      ],
    );
  }
}

class _InvoiceTile extends StatelessWidget {
  const _InvoiceTile({required this.inv});
  final InvoiceModel inv;

  @override
  Widget build(BuildContext context) {
    final fmt = NumberFormat.simpleCurrency(name: inv.currencyCode);
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: IspSpacing.lg),
      child: Card(
        child: InkWell(
          borderRadius: BorderRadius.circular(IspRadii.lg),
          onTap: () => Navigator.pushNamed(context, '/invoices/${inv.id}'),
          child: Padding(
            padding: const EdgeInsets.all(IspSpacing.lg),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      inv.invoiceNumber,
                      style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
                    ),
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
                const SizedBox(height: 6),
                Text(
                  'Jatuh tempo ${dateFmt.format(inv.dueDate)}',
                  style: const TextStyle(fontSize: 12, color: IspColors.textTertiary),
                ),
                const SizedBox(height: 12),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      fmt.format(inv.amount),
                      style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w700),
                    ),
                    if (!inv.isPaid)
                      ElevatedButton(
                        onPressed: () {},
                        child: const Text('Bayar'),
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
