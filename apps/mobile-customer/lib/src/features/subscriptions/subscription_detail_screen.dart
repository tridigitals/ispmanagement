import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../../l10n/app_localizations.dart';
import '../../../services/service_providers.dart';

final subscriptionByIdProvider =
    FutureProvider.family<SubscriptionModel, String>((ref, id) async {
  final svc = ref.watch(subscriptionServiceProvider);
  final res = await svc.getById(id);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw exception.message,
  };
});

class SubscriptionDetailScreen extends ConsumerWidget {
  const SubscriptionDetailScreen({required this.id, super.key});
  final String id;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');
    final subAsync = ref.watch(subscriptionByIdProvider(id));
    return Scaffold(
      appBar: AppBar(title: const Text('Detail Langganan')),
      body: subAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text(e.toString())),
        data: (sub) => ListView(
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
                        Text(sub.packageName ?? 'Paket',
                            style: Theme.of(context).textTheme.titleLarge),
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
                    const SizedBox(height: 16),
                    Text(
                      '${fmt.format(sub.price)} / ${sub.billingCycle}',
                      style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w700),
                    ),
                    const Divider(height: 32),
                    _InfoRow(label: 'Lokasi', value: sub.locationLabel ?? '-'),
                    _InfoRow(label: 'Router', value: sub.routerName ?? '-'),
                    if (sub.startsAt != null)
                      _InfoRow(label: 'Mulai', value: dateFmt.format(sub.startsAt!)),
                    if (sub.endsAt != null)
                      _InfoRow(label: 'Berakhir', value: dateFmt.format(sub.endsAt!)),
                    if (sub.graceUntil != null)
                      _InfoRow(
                        label: 'Masa tenggang',
                        value: dateFmt.format(sub.graceUntil!),
                      ),
                    if (sub.notes != null && sub.notes!.isNotEmpty)
                      _InfoRow(label: 'Catatan', value: sub.notes!),
                  ],
                ),
              ),
            ),
            const SizedBox(height: IspSpacing.lg),
            OutlinedButton.icon(
              onPressed: () => Navigator.pushNamed(context, '/tickets/new'),
              icon: const Icon(Icons.report_problem_outlined),
              label: const Text('Lapor Gangguan'),
            ),
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
