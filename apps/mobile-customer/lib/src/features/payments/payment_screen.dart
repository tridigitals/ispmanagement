import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../../l10n/app_localizations.dart';
import '../../../services/payment_providers.dart';

/// Payment method picker → creates transaction → shows payment instructions.
class PaymentScreen extends ConsumerWidget {
  const PaymentScreen({required this.invoiceId, super.key});
  final String invoiceId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final channelsAsync = ref.watch(paymentChannelsProvider(invoiceId));
    final l10n = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(title: Text(l10n.choosePaymentMethod)),
      body: channelsAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text(e.toString())),
        data: (channels) {
          // Group by method.
          final grouped = <PaymentMethod, List<PaymentChannel>>{};
          for (final c in channels) {
            grouped.putIfAbsent(c.method, () => []).add(c);
          }
          return ListView(
            padding: const EdgeInsets.all(IspSpacing.lg),
            children: [
              for (final method in grouped.keys) ...[
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 8),
                  child: Text(
                    method.label,
                    style: const TextStyle(
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                      color: IspColors.textTertiary,
                    ),
                  ),
                ),
                for (final ch in grouped[method]!)
                  _ChannelTile(
                    channel: ch,
                    onTap: () => _createTransaction(context, ref, ch),
                  ),
                const SizedBox(height: 12),
              ],
            ],
          );
        },
      ),
    );
  }

  Future<void> _createTransaction(
    BuildContext context,
    WidgetRef ref,
    PaymentChannel ch,
  ) async {
    final res = await ref.read(createTransactionProvider(invoiceId).future);
    res.when(
      success: (txn) {
        if (context.mounted) {
          context.push('/invoices/$invoiceId/payment/${txn.id}');
        }
      },
      failure: (e) {
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(e.message)),
          );
        }
      },
    );
  }
}

class _ChannelTile extends StatelessWidget {
  const _ChannelTile({required this.channel, required this.onTap});
  final PaymentChannel channel;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    return Card(
      margin: const EdgeInsets.symmetric(vertical: 4),
      child: InkWell(
        borderRadius: BorderRadius.circular(IspRadii.lg),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Container(
                width: 48,
                height: 48,
                decoration: BoxDecoration(
                  color: IspColors.bgTertiary,
                  borderRadius: BorderRadius.circular(IspRadii.md),
                ),
                alignment: Alignment.center,
                child: Text(
                  channel.name.substring(0, channel.name.length > 2 ? 2 : channel.name.length).toUpperCase(),
                  style: const TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      channel.name,
                      style: const TextStyle(
                        fontSize: 14,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    if (channel.fee > 0)
                      Text(
                        '+ biaya ${fmt.format(channel.fee)}',
                        style: const TextStyle(
                          fontSize: 11,
                          color: IspColors.textTertiary,
                        ),
                      ),
                  ],
                ),
              ),
              const Icon(Icons.chevron_right, color: IspColors.textTertiary),
            ],
          ),
        ),
      ),
    );
  }
}
