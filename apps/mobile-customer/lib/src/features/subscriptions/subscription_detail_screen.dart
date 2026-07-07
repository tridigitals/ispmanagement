import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';
import '../../services/missing_providers.dart';

// ── Provider ──────────────────────────────────────────────

final subscriptionByIdProvider =
    FutureProvider.family<SubscriptionModel, String>((ref, id) async {
  final svc = ref.watch(subscriptionServiceProvider);
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
    final subAsync = ref.watch(subscriptionByIdProvider(id));

    return subAsync.when(
      loading: () => Scaffold(
        body: Center(child: CircularProgressIndicator(color: isp.accent)),
      ),
      error: (e, _) => Scaffold(
        appBar: AppBar(),
        body: _ErrorView(
          message: e.toString(),
          onRetry: () => ref.invalidate(subscriptionByIdProvider(id)),
        ),
      ),
      data: (sub) => _DetailBody(sub: sub),
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
        OutlinedButton.icon(
          onPressed: onRetry,
          icon: const Icon(Icons.refresh),
          label: const Text('Coba Lagi'),
        ),
      ]),
    );
  }
}

// ── Detail Body ───────────────────────────────────────────

class _DetailBody extends ConsumerWidget {
  const _DetailBody({required this.sub});
  final SubscriptionModel sub;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');
    final remaining =
        sub.endsAt != null ? sub.endsAt!.difference(DateTime.now()).inDays : 0;
    final statusColor = sub.isActive ? isp.success : isp.danger;

    return Scaffold(
      body: SafeArea(
        child: RefreshIndicator(
          color: isp.accent,
          onRefresh: () async {
            ref.invalidate(subscriptionByIdProvider(sub.id));
            await ref.read(subscriptionByIdProvider(sub.id).future);
          },
          child: CustomScrollView(
            slivers: [
              // ── AppBar ──
              _DetailAppBar(sub: sub, statusColor: statusColor),
              // ── Content ──
              SliverPadding(
                padding: const EdgeInsets.fromLTRB(16, 0, 16, 100),
                sliver: SliverList(
                  delegate: SliverChildListDelegate([
                    const SizedBox(height: 8),

                    // ── 1. Ringkasan — harga + sisa hari ──
                    _SummaryCard(
                      sub: sub,
                      remaining: remaining,
                      statusColor: statusColor,
                      fmt: fmt,
                      dateFmt: dateFmt,
                    ),

                    const SizedBox(height: 16),

                    // ── 2. Info detail ──
                    _InfoCard(sub: sub, dateFmt: dateFmt),

                    const SizedBox(height: 16),

                    // ── 3. Tindakan ──
                    _ActionButtons(sub: sub),

                    const SizedBox(height: 24),

                    // ── 4. Riwayat tagihan ──
                    _InvoiceHistory(sub: sub),
                  ]),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ── AppBar ────────────────────────────────────────────────

class _DetailAppBar extends StatelessWidget {
  const _DetailAppBar({required this.sub, required this.statusColor});
  final SubscriptionModel sub;
  final Color statusColor;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return SliverAppBar(
      pinned: true,
      backgroundColor: isp.background,
      leading: GestureDetector(
        onTap: () => GoRouter.of(context).pop(),
        child: Container(
          margin: const EdgeInsets.only(left: 12),
          width: 40,
          height: 40,
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
          Text(
            sub.packageName ?? 'Paket',
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w700,
              color: isp.textPrimary,
            ),
          ),
          Text(
            sub.id,
            style: TextStyle(fontSize: 11, color: isp.textMuted),
          ),
        ],
      ),
    );
  }
}

// ── 1. Summary Card — harga besar + sisa hari ─────────────

class _SummaryCard extends StatelessWidget {
  const _SummaryCard({
    required this.sub,
    required this.remaining,
    required this.statusColor,
    required this.fmt,
    required this.dateFmt,
  });
  final SubscriptionModel sub;
  final int remaining;
  final Color statusColor;
  final NumberFormat fmt;
  final DateFormat dateFmt;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final progress =
        sub.endsAt != null && sub.startsAt != null
            ? _computeProgress(sub.startsAt!, sub.endsAt!)
            : 0.0;

    return Container(
      decoration: _nbCard(isp),
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Status badge
          Row(
            children: [
              Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 4,
                ),
                decoration: BoxDecoration(
                  color: statusColor.withOpacity(0.12),
                  borderRadius: BorderRadius.circular(999),
                ),
                child: Text(
                  sub.statusLabel().toUpperCase(),
                  style: TextStyle(
                    fontSize: 10,
                    fontWeight: FontWeight.w800,
                    color: statusColor,
                    letterSpacing: 1,
                  ),
                ),
              ),
              const Spacer(),
              // Sisa hari
              if (sub.isActive)
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.hourglass_bottom_rounded,
                      size: 14,
                      color: isp.textMuted,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      '$remaining hari lagi',
                      style: TextStyle(
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                        color: remaining > 7 ? isp.success : isp.warning,
                      ),
                    ),
                  ],
                ),
            ],
          ),
          const SizedBox(height: 16),

          // Harga besar
          Text(
            fmt.format(sub.price),
            style: TextStyle(
              fontSize: 32,
              fontWeight: FontWeight.w900,
              color: isp.textPrimary,
              letterSpacing: -2,
              height: 1.0,
            ),
          ),
          const SizedBox(height: 2),
          Text(
            '/ ${_billingLabel(sub.billingCycle)}',
            style: TextStyle(fontSize: 13, color: isp.textMuted),
          ),

          // Progress bar
          const SizedBox(height: 16),
          ClipRRect(
            borderRadius: BorderRadius.circular(2),
            child: LinearProgressIndicator(
              value: progress,
              backgroundColor: isp.border,
              color: statusColor,
              minHeight: 3,
            ),
          ),
          const SizedBox(height: 6),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              if (sub.startsAt != null)
                Text(
                  dateFmt.format(sub.startsAt!),
                  style: TextStyle(fontSize: 10, color: isp.textMuted),
                ),
              if (sub.endsAt != null)
                Text(
                  dateFmt.format(sub.endsAt!),
                  style: TextStyle(fontSize: 10, color: isp.textMuted),
                ),
            ],
          ),
        ],
      ),
    );
  }

  static double _computeProgress(DateTime start, DateTime end) {
    final total = end.difference(start).inDays;
    final elapsed = DateTime.now().difference(start).inDays;
    if (total <= 0) return 0.0;
    return (elapsed / total).clamp(0.0, 1.0);
  }
}

// ── 2. Info Card — detail koneksi & tagihan ───────────────

class _InfoCard extends StatelessWidget {
  const _InfoCard({required this.sub, required this.dateFmt});
  final SubscriptionModel sub;
  final DateFormat dateFmt;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');

    final rows = <_InfoRow>[
      _InfoRow(icon: Icons.receipt_long_outlined, label: 'Siklus', value: _billingLabel(sub.billingCycle)),
      _InfoRow(icon: Icons.payments_outlined, label: 'Harga', value: '${fmt.format(sub.price)} / ${sub.billingCycle}'),
      if (sub.locationLabel != null)
        _InfoRow(icon: Icons.location_on_outlined, label: 'Lokasi', value: sub.locationLabel!),
      if (sub.routerName != null)
        _InfoRow(icon: Icons.router, label: 'Router', value: sub.routerName!),
      if (sub.startsAt != null)
        _InfoRow(icon: Icons.event_available_outlined, label: 'Mulai', value: dateFmt.format(sub.startsAt!)),
      if (sub.endsAt != null)
        _InfoRow(icon: Icons.event_busy_outlined, label: 'Berakhir', value: dateFmt.format(sub.endsAt!)),
      if (sub.graceUntil != null)
        _InfoRow(icon: Icons.hourglass_bottom_outlined, label: 'Masa Tenggang', value: dateFmt.format(sub.graceUntil!)),
      if (sub.notes != null && sub.notes!.isNotEmpty)
        _InfoRow(icon: Icons.note_outlined, label: 'Catatan', value: sub.notes!),
    ];

    return Container(
      decoration: _nbCard(isp),
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Detail Layanan',
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w700,
              color: isp.textSecondary,
              letterSpacing: 0.5,
            ),
          ),
          const SizedBox(height: 12),
          ...rows.map((r) => r),
        ],
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({
    required this.icon,
    required this.label,
    required this.value,
  });
  final IconData icon;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        children: [
          Icon(icon, size: 16, color: isp.textMuted),
          const SizedBox(width: 10),
          SizedBox(
            width: 100,
            child: Text(
              label,
              style: TextStyle(fontSize: 13, color: isp.textMuted),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w600,
                color: isp.textPrimary,
              ),
              textAlign: TextAlign.right,
            ),
          ),
        ],
      ),
    );
  }
}

// ── 3. Action Buttons ─────────────────────────────────────

class _ActionButtons extends StatelessWidget {
  const _ActionButtons({required this.sub});
  final SubscriptionModel sub;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Column(
      children: [
        SizedBox(
          width: double.infinity,
          child: _ActionBtn(
            icon: Icons.report_problem_outlined,
            label: 'Lapor Gangguan',
            color: isp.warning,
            onTap: () => GoRouter.of(context).push('/tickets/new'),
          ),
        ),
      ],
    );
  }
}

class _ActionBtn extends StatelessWidget {
  const _ActionBtn({
    required this.icon,
    required this.label,
    required this.color,
    required this.onTap,
  });
  final IconData icon;
  final String label;
  final Color color;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(14),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          decoration: BoxDecoration(
            color: isp.surface,
            borderRadius: BorderRadius.circular(14),
            border: Border.all(color: isp.border, width: 1.5),
            boxShadow: [
              BoxShadow(
                color: isp.border.withOpacity(0.5),
                offset: const Offset(3, 3),
                blurRadius: 0,
              ),
            ],
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(icon, size: 18, color: color),
              const SizedBox(width: 8),
              Text(
                label,
                style: TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w700,
                  color: isp.textPrimary,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ── 4. Invoice History ────────────────────────────────────

class _InvoiceHistory extends ConsumerWidget {
  const _InvoiceHistory({required this.sub});
  final SubscriptionModel sub;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final async = ref.watch(subscriptionInvoicesProvider(sub.id));

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(
              'Riwayat Tagihan',
              style: TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w700,
                color: isp.textPrimary,
              ),
            ),
            const Spacer(),
            GestureDetector(
              onTap: () =>
                  GoRouter.of(context).push('/subscriptions/${sub.id}/invoices'),
              child: Text(
                'Lihat Semua',
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                  color: isp.accent,
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        async.when(
          loading: () => const Center(
            child: Padding(
              padding: EdgeInsets.all(24),
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
          ),
          error: (_, __) => Text(
            'Gagal memuat',
            style: TextStyle(fontSize: 13, color: isp.textMuted),
          ),
          data: (invoices) {
            if (invoices.isEmpty) {
              return Container(
                padding: const EdgeInsets.all(24),
                decoration: _nbCard(isp),
                child: Center(
                  child: Text(
                    'Belum ada tagihan',
                    style: TextStyle(fontSize: 13, color: isp.textMuted),
                  ),
                ),
              );
            }
            return Column(
              children:
                  invoices.take(3).map((inv) => _InvoiceRow(invoice: inv)).toList(),
            );
          },
        ),
      ],
    );
  }
}

class _InvoiceRow extends StatelessWidget {
  const _InvoiceRow({required this.invoice});
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
          borderRadius: BorderRadius.circular(14),
          child: Container(
            decoration: _nbCard(isp),
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        invoice.invoiceNumber ?? invoice.id,
                        style: TextStyle(
                          fontSize: 13,
                          fontWeight: FontWeight.w600,
                          color: isp.textPrimary,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        dateFmt.format(invoice.dueDate ?? DateTime.now()),
                        style: TextStyle(fontSize: 11, color: isp.textMuted),
                      ),
                    ],
                  ),
                ),
                Text(
                  fmt.format(invoice.amount),
                  style: TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w700,
                    color: isp.textPrimary,
                  ),
                ),
                const SizedBox(width: 10),
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 3,
                  ),
                  decoration: BoxDecoration(
                    color:
                        isPaid
                            ? isp.success.withOpacity(0.12)
                            : isp.warning.withOpacity(0.12),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Text(
                    isPaid ? 'Lunas' : 'Jatuh Tempo',
                    style: TextStyle(
                      fontSize: 10,
                      fontWeight: FontWeight.w700,
                      color: isPaid ? isp.success : isp.warning,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// ── Helpers ───────────────────────────────────────────────

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

String _billingLabel(String cycle) {
  switch (cycle.toLowerCase()) {
    case 'monthly':
    case 'bulan':
      return 'Bulanan';
    case 'quarterly':
      return 'Per 3 Bulan';
    case 'yearly':
    case 'tahun':
      return 'Tahunan';
    default:
      return cycle;
  }
}
