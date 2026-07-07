import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';
import '../../services/missing_providers.dart';

// ── Provider ──────────────────────────────────────────────

final subscriptionByIdProvider =
    FutureProvider.family<SubscriptionModel, String>((ref, id) async {
  final svc = ref.watch(subscriptionServiceProvider);
  // Backend has no GET /my-subscriptions/{id} — fetch from list and filter
  final ServiceResult<PaginatedResponse<SubscriptionModel>> res =
      await svc.list(page: 1, perPage: 100);
  return switch (res) {
    Success(:final data) => data.data.firstWhere(
        (s) => s.id == id,
        orElse: () => throw Exception('Paket tidak ditemukan'),
      ),
    Failure(:final exception) => throw exception.message,
  };
});

// ── Screen ────────────────────────────────────────────────

class SubscriptionDetailScreen extends ConsumerWidget {
  const SubscriptionDetailScreen({required this.id, super.key});
  final String id;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final subAsync = ref.watch(subscriptionByIdProvider(id));

    return subAsync.when(
      loading: () => Scaffold(
        body: Center(child: CircularProgressIndicator(color: isp.accent)),
      ),
      error: (e, _) => Scaffold(
        appBar: AppBar(),
        body: _ErrorView(message: e.toString(), onRetry: () => ref.invalidate(subscriptionByIdProvider(id))),
      ),
      data: (sub) => _DetailBody(sub: sub, ref: ref),
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.message, required this.onRetry});
  final String message;
  final VoidCallback onRetry;
  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Center(
      child: Column(mainAxisSize: MainAxisSize.min, children: [
        Icon(Icons.error_outline, size: 48, color: isp.danger),
        const SizedBox(height: 16),
        Text(message, textAlign: TextAlign.center),
        const SizedBox(height: 16),
        OutlinedButton.icon(onPressed: onRetry, icon: const Icon(Icons.refresh), label: const Text('Coba Lagi')),
      ]),
    );
  }
}

class _DetailBody extends ConsumerWidget {
  const _DetailBody({required this.sub, required this.ref});
  final SubscriptionModel sub;
  final WidgetRef ref;

  @override
  Widget build(BuildContext context, WidgetRef ref_) {
    final isp = context.isp;
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');

    // Compute remaining days
    final remaining = sub.endsAt != null ? sub.endsAt!.difference(DateTime.now()).inDays : 0;
    final statusColor = sub.isActive ? isp.success : isp.danger;

    return Scaffold(
      body: SafeArea(
        child: RefreshIndicator(
          color: isp.accent,
          onRefresh: () async {
            ref_.invalidate(subscriptionByIdProvider(sub.id));
            await ref_.read(subscriptionByIdProvider(sub.id).future);
          },
          child: CustomScrollView(
            slivers: [
              // ── Header with back button ──
              SliverAppBar(
                pinned: true,
                backgroundColor: isp.background,
                leading: GestureDetector(
                  onTap: () => GoRouter.of(context).pop(),
                  child: Container(
                    margin: const EdgeInsets.only(left: 12),
                    width: 40, height: 40,
                    decoration: BoxDecoration(
                      color: isp.surface,
                      borderRadius: BorderRadius.circular(12),
                      border: Border.all(color: isp.border, width: 1.5),
                    ),
                    child: const Icon(Icons.arrow_back_rounded, size: 20),
                  ),
                ),
                title: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(sub.packageName ?? 'Paket', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: isp.textPrimary)),
                    Text(sub.id, style: TextStyle(fontSize: 11, color: isp.textMuted)),
                  ],
                ),
              ),
              SliverPadding(
                padding: const EdgeInsets.fromLTRB(16, 0, 16, 100),
                sliver: SliverList(delegate: SliverChildListDelegate([
                  const SizedBox(height: 8),
                  // ── Hero card w/ countdown ──
                  _HeroCard(sub: sub, remaining: remaining, statusColor: statusColor, fmt: fmt, dateFmt: dateFmt),
                  const SizedBox(height: 16),
                  // ── Stats grid (2x2) ──
                  _StatsGrid(sub: sub),
                  const SizedBox(height: 24),
                  // ── Riwayat Tagihan header ──
                  Text('Riwayat Tagihan', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w700, color: isp.textPrimary)),
                  const SizedBox(height: 12),
                  // ── Invoice history ──
                  _InvoiceHistory(sub: sub, ref: ref_),
                ])),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ── Hero Card (left content + right countdown ring) ──────

class _HeroCard extends StatelessWidget {
  const _HeroCard({required this.sub, required this.remaining, required this.statusColor, required this.fmt, required this.dateFmt});
  final SubscriptionModel sub;
  final int remaining;
  final Color statusColor;
  final NumberFormat fmt;
  final DateFormat dateFmt;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;

    return Container(
      decoration: BoxDecoration(
        color: isp.surface, borderRadius: BorderRadius.circular(20),
        border: Border.all(color: isp.border, width: 1.5),
      ),
      padding: const EdgeInsets.all(16),
      child: IntrinsicHeight(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Left: package info
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                    decoration: BoxDecoration(
                      color: statusColor.withOpacity(0.12), borderRadius: BorderRadius.circular(999),
                    ),
                    child: Text(sub.statusLabel().toUpperCase(), style: TextStyle(fontSize: 10, fontWeight: FontWeight.w800, color: statusColor, letterSpacing: 1)),
                  ),
                  const SizedBox(height: 12),
                  Text(fmt.format(sub.price), style: TextStyle(fontSize: 28, fontWeight: FontWeight.w900, color: isp.textPrimary, letterSpacing: -2, height: 1.0)),
                  const SizedBox(height: 2),
                  Text('/ ${sub.billingCycle}', style: TextStyle(fontSize: 12, color: isp.textMuted)),
                  if (sub.endsAt != null) ...[
                    const SizedBox(height: 8),
                    Row(children: [
                      Icon(Icons.calendar_today_outlined, size: 12, color: isp.textMuted),
                      const SizedBox(width: 4),
                      Text('Jatuh tempo ${dateFmt.format(sub.endsAt!)}', style: TextStyle(fontSize: 11, color: isp.textMuted)),
                    ]),
                  ],
                ],
              ),
            ),
            const SizedBox(width: 12),
            // Right: countdown ring
            SizedBox(
              width: 80, height: 80,
              child: Stack(
                fit: StackFit.expand,
                children: [
                  CircularProgressIndicator(
                    value: (remaining / 30).clamp(0.0, 1.0),
                    strokeWidth: 4, backgroundColor: isp.border,
                    color: remaining > 7 ? isp.success : isp.warning,
                  ),
                  Center(
                    child: Column(mainAxisSize: MainAxisSize.min, children: [
                      Text('$remaining', style: TextStyle(fontSize: 20, fontWeight: FontWeight.w800, color: isp.textPrimary)),
                      Text('hari lagi', style: TextStyle(fontSize: 10, color: isp.textMuted)),
                    ]),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ── Stats grid 2×2 (ponytail: hardcoded placeholders) ────

class _StatsGrid extends StatelessWidget {
  const _StatsGrid({required this.sub});
  final SubscriptionModel sub;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;

    final items = [
      _StatItem(icon: Icons.download_rounded, value: '50 Mbps', label: 'Kecepatan', color: isp.accent),
      _StatItem(icon: Icons.check_circle_outline, value: '99.8%', label: 'Uptime', color: isp.success),
      _StatItem(icon: Icons.speed, value: '12 ms', label: 'Latensi', color: isp.info),
      _StatItem(icon: Icons.cloud_download_outlined, value: '128 GB', label: 'Data Used', color: isp.warning),
    ];

    return Row(
      children: [
        Expanded(child: _StatCard(item: items[0])),
        const SizedBox(width: 8),
        Expanded(child: _StatCard(item: items[1])),
        const SizedBox(width: 8),
        Expanded(child: _StatCard(item: items[2])),
        const SizedBox(width: 8),
        Expanded(child: _StatCard(item: items[3])),
      ],
    );
  }
}

class _StatItem {
  const _StatItem({required this.icon, required this.value, required this.label, required this.color});
  final IconData icon;
  final String value;
  final String label;
  final Color color;
}

class _StatCard extends StatelessWidget {
  const _StatCard({required this.item});
  final _StatItem item;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Container(
      padding: const EdgeInsets.symmetric(vertical: 14, horizontal: 8),
      decoration: BoxDecoration(
        color: isp.surface, borderRadius: BorderRadius.circular(16),
        border: Border.all(color: isp.border, width: 1.5),
      ),
      child: Column(children: [
        Icon(item.icon, size: 20, color: item.color),
        const SizedBox(height: 8),
        Text(item.value, style: TextStyle(fontSize: 13, fontWeight: FontWeight.w800, color: isp.textPrimary), textAlign: TextAlign.center),
        const SizedBox(height: 2),
        Text(item.label, style: TextStyle(fontSize: 9, color: isp.textMuted), textAlign: TextAlign.center),
      ]),
    );
  }
}

// ── Invoice history cards ─────────────────────────────────

class _InvoiceHistory extends ConsumerWidget {
  const _InvoiceHistory({required this.sub, required this.ref});
  final SubscriptionModel sub;
  final WidgetRef ref;

  @override
  Widget build(BuildContext context, WidgetRef ref_) {
    final async = ref_.watch(subscriptionInvoicesProvider(sub.id));

    return async.when(
      loading: () => const Center(child: Padding(padding: EdgeInsets.all(24), child: CircularProgressIndicator(strokeWidth: 2))),
      error: (e, _) => Text('Gagal memuat', style: TextStyle(color: context.isp.textMuted)),
      data: (invoices) {
        if (invoices.isEmpty) {
          return Text('Belum ada tagihan', style: TextStyle(fontSize: 13, color: context.isp.textMuted));
        }
        return Column(
          children: invoices.take(3).map((inv) => _InvoiceCard(invoice: inv)).toList(),
        );
      },
    );
  }
}

class _InvoiceCard extends StatelessWidget {
  const _InvoiceCard({required this.invoice});
  final InvoiceModel invoice;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final dateFmt = DateFormat('MMM yyyy', 'id_ID');
    final isPaid = invoice.status == InvoiceStatus.paid;

    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () => GoRouter.of(context).push('/invoices/${invoice.id}'),
          borderRadius: BorderRadius.circular(16),
          child: Container(
            decoration: BoxDecoration(
              color: isp.surface, borderRadius: BorderRadius.circular(16),
              border: Border.all(color: isp.border, width: 1.5),
            ),
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
            child: Row(children: [
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text(invoice.invoiceNumber ?? invoice.id, style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: isp.textPrimary)),
                  const SizedBox(height: 2),
                  Text(dateFmt.format(invoice.dueDate ?? DateTime.now()), style: TextStyle(fontSize: 11, color: isp.textMuted)),
                ]),
              ),
              Text(fmt.format(invoice.amount), style: TextStyle(fontSize: 14, fontWeight: FontWeight.w700, color: isp.textPrimary)),
              const SizedBox(width: 10),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                decoration: BoxDecoration(
                  color: isPaid ? isp.success.withOpacity(0.12) : isp.warning.withOpacity(0.12),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(isPaid ? 'Lunas' : 'Jatuh Tempo', style: TextStyle(fontSize: 10, fontWeight: FontWeight.w700, color: isPaid ? isp.success : isp.warning)),
              ),
            ]),
          ),
        ),
      ),
    );
  }
}
