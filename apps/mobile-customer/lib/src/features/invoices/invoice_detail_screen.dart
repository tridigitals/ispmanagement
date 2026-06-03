import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../../l10n/app_localizations.dart';
import '../../../services/service_providers.dart';

class InvoiceDetailScreen extends ConsumerWidget {
  const InvoiceDetailScreen({required this.id, super.key});
  final String id;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final dateFmt = DateFormat('d MMMM yyyy', 'id_ID');
    final invAsync = ref.watch(invoiceByIdProvider(id));

    return Scaffold(
      appBar: AppBar(title: const Text('Detail Tagihan')),
      body: invAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text(e.toString())),
        data: (inv) => ListView(
          padding: const EdgeInsets.all(IspSpacing.lg),
          children: [
            Card(
              child: Padding(
                padding: const EdgeInsets.all(IspSpacing.xl),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text(inv.invoiceNumber, style: Theme.of(context).textTheme.titleMedium),
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
                    const SizedBox(height: 24),
                    Text(
                      fmt.format(inv.amount),
                      style: const TextStyle(fontSize: 32, fontWeight: FontWeight.w800),
                    ),
                    const SizedBox(height: 24),
                    _InfoRow(label: 'Jatuh tempo', value: dateFmt.format(inv.dueDate)),
                    if (inv.paidAt != null)
                      _InfoRow(label: 'Dibayar pada', value: dateFmt.format(inv.paidAt!)),
                    if (inv.subscriptionLabel != null)
                      _InfoRow(label: 'Layanan', value: inv.subscriptionLabel!),
                    if (inv.notes != null && inv.notes!.isNotEmpty)
                      _InfoRow(label: 'Catatan', value: inv.notes!),
                  ],
                ),
              ),
            ),
            const SizedBox(height: IspSpacing.lg),
            if (!inv.isPaid) ...[
              ElevatedButton.icon(
                onPressed: () => context.push('/invoices/${inv.id}/pay'),
                icon: const Icon(Icons.payment),
                label: const Text('Bayar Sekarang'),
              ),
              const SizedBox(height: 8),
              OutlinedButton.icon(
                onPressed: () => context.push('/tickets/new'),
                icon: const Icon(Icons.help_outline),
                label: const Text('Butuh Bantuan?'),
              ),
            ] else ...[
              OutlinedButton.icon(
                onPressed: () {},
                icon: const Icon(Icons.download),
                label: const Text('Download Struk'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});
  final String label;
  final String value;
  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 120,
            child: Text(label, style: const TextStyle(color: IspColors.textTertiary)),
          ),
          Expanded(child: Text(value)),
        ],
      ),
    );
  }
}
