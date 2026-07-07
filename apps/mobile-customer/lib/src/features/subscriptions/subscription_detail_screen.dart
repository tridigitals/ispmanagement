import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';

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
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');
    final subAsync = ref.watch(subscriptionByIdProvider(id));

    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.subscriptionDetail ?? 'Detail Langganan'),
      ),
      body: subAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Padding(
            padding: const EdgeInsets.all(IspSpacing.xl),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  Icons.error_outline,
                  size: 48,
                  color: isp.danger,
                ),
                const SizedBox(height: IspSpacing.md),
                Text(e.toString(), textAlign: TextAlign.center),
                const SizedBox(height: IspSpacing.lg),
                OutlinedButton.icon(
                  onPressed: () => ref.invalidate(subscriptionByIdProvider(id)),
                  icon: const Icon(Icons.refresh),
                  label: Text(l10n.retry ?? 'Coba Lagi'),
                ),
              ],
            ),
          ),
        ),
        data: (sub) => RefreshIndicator(
          onRefresh: () async {
            ref.invalidate(subscriptionByIdProvider(id));
            await ref.read(subscriptionByIdProvider(id).future);
          },
          child: ListView(
            padding: const EdgeInsets.only(
              left: IspSpacing.lg,
              right: IspSpacing.lg,
              top: IspSpacing.lg,
              bottom: IspSpacing.xxxl,
            ),
            children: [
              // ── Hero header ──────────────────────────────
              _HeroHeader(sub: sub, fmt: fmt, l10n: l10n),
              const SizedBox(height: IspSpacing.lg),

              // ── Speed section ────────────────────────────
              _SpeedSection(sub: sub, l10n: l10n),
              const SizedBox(height: IspSpacing.lg),

              // ── Connection details ───────────────────────
              _SectionCard(
                title: l10n.connectionDetails ?? 'Detail Koneksi',
                icon: Icons.settings_ethernet,
                children: [
                  _DetailRow(
                    icon: Icons.router,
                    label: l10n.router ?? 'Router',
                    value: sub.routerName ?? '-',
                  ),
                  _DetailRow(
                    icon: Icons.location_on_outlined,
                    label: l10n.location ?? 'Lokasi',
                    value: sub.locationLabel ?? '-',
                  ),
                  if (sub.notes != null && sub.notes!.isNotEmpty)
                    _DetailRow(
                      icon: Icons.note_outlined,
                      label: l10n.notes ?? 'Catatan',
                      value: sub.notes!,
                    ),
                ],
              ),
              const SizedBox(height: IspSpacing.lg),

              // ── Billing info ─────────────────────────────
              _SectionCard(
                title: l10n.billingInfo ?? 'Informasi Tagihan',
                icon: Icons.receipt_long_outlined,
                children: [
                  _DetailRow(
                    icon: Icons.payments_outlined,
                    label: l10n.price ?? 'Harga',
                    value: '${fmt.format(sub.price)} / ${sub.billingCycle}',
                  ),
                  _DetailRow(
                    icon: Icons.calendar_today_outlined,
                    label: l10n.cycle ?? 'Siklus',
                    value: _billingCycleLabel(sub.billingCycle),
                  ),
                  if (sub.startsAt != null)
                    _DetailRow(
                      icon: Icons.event_available_outlined,
                      label: l10n.startsAt ?? 'Mulai',
                      value: dateFmt.format(sub.startsAt!),
                    ),
                  if (sub.endsAt != null)
                    _DetailRow(
                      icon: Icons.event_busy_outlined,
                      label: l10n.endsAt ?? 'Berakhir',
                      value: dateFmt.format(sub.endsAt!),
                    ),
                  if (sub.graceUntil != null)
                    _DetailRow(
                      icon: Icons.hourglass_bottom_outlined,
                      label: l10n.gracePeriod ?? 'Masa tenggang',
                      value: dateFmt.format(sub.graceUntil!),
                    ),
                ],
              ),
              const SizedBox(height: IspSpacing.sm),
              SizedBox(
                width: double.infinity,
                child: OutlinedButton.icon(
                  onPressed: () => GoRouter.of(context)
                      .push('/subscriptions/${sub.id}/invoices'),
                  icon: const Icon(Icons.receipt_long_outlined, size: 18),
                  label: const Text('Lihat Tagihan'),
                ),
              ),
              const SizedBox(height: IspSpacing.xl),

              // ── Actions ──────────────────────────────────
              if (sub.status == SubscriptionStatus.pendingInstallation) ...[
                _InstallationTracker(sub: sub, l10n: l10n),
                const SizedBox(height: IspSpacing.lg),
              ],
              SizedBox(
                width: double.infinity,
                child: OutlinedButton.icon(
                  onPressed: () =>
                      GoRouter.of(context).push('/tickets/new'),
                  icon: const Icon(Icons.report_problem_outlined),
                  label: Text(l10n.reportOutage ?? 'Lapor Gangguan'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  static String _billingCycleLabel(String cycle) {
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
}

// ── Hero header with gradient ─────────────────────────────

class _HeroHeader extends StatelessWidget {
  const _HeroHeader({
    required this.sub,
    required this.fmt,
    required this.l10n,
  });
  final SubscriptionModel sub;
  final NumberFormat fmt;
  final AppLocalizations l10n;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final statusColor = _statusTone() == StatusTone.success
        ? isp.success
        : _statusTone() == StatusTone.danger
            ? isp.danger
            : isp.warning;

    return Container(
      padding: const EdgeInsets.all(IspSpacing.xl),
      decoration: BoxDecoration(
        color: isp.surface,
        borderRadius: BorderRadius.circular(IspRadii.xl),
        border: Border.all(color: isp.border),
      ),
      child: Column(
        children: [
          // ── Circular progress indicator ──
          SizedBox(
            width: 140,
            height: 140,
            child: Stack(
              fit: StackFit.expand,
              children: [
                CustomPaint(
                  painter: _CircularProgressPainter(
                    progress: _daysProgress(),
                    color: statusColor,
                    bgColor: isp.border,
                    strokeWidth: 8,
                  ),
                ),
                Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        fmt.format(sub.price),
                        style: TextStyle(
                          fontSize: 24,
                          fontWeight: FontWeight.w800,
                          color: isp.textPrimary,
                          height: 1.0,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        '/ ${sub.billingCycle}',
                        style: TextStyle(
                          fontSize: 12,
                          color: isp.textMuted,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: IspSpacing.lg),
          // Package name + status
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text(
                  sub.packageName ?? (l10n.internetPackage ?? 'Paket Internet'),
                  style: TextStyle(
                    color: isp.textPrimary,
                    fontSize: 16,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              IspStatusBadge(
                label: sub.statusLabel(),
                tone: _statusTone(),
                icon: _statusIcon(),
              ),
            ],
          ),
          // Router + Location chips
          if (sub.routerName != null || sub.locationLabel != null) ...[
            const SizedBox(height: IspSpacing.md),
            Wrap(
              spacing: IspSpacing.sm,
              runSpacing: IspSpacing.xs,
              children: [
                if (sub.routerName != null)
                  _InfoChip(
                    icon: Icons.router,
                    label: sub.routerName!,
                  ),
                if (sub.locationLabel != null)
                  _InfoChip(
                    icon: Icons.location_on_outlined,
                    label: sub.locationLabel!,
                  ),
              ],
            ),
          ],
        ],
      ),
    );
  }

  /// Returns progress as 0.0–1.0 based on days elapsed between startAt and endAt.
  double _daysProgress() {
    final start = sub.startsAt;
    final end = sub.endsAt;
    if (start == null || end == null) return 0.0;
    final total = end.difference(start).inDays;
    final elapsed = DateTime.now().difference(start).inDays;
    if (total <= 0) return 0.0;
    return (elapsed / total).clamp(0.0, 1.0);
  }

  StatusTone _statusTone() {
    if (sub.isActive) return StatusTone.success;
    if (sub.needsAttention) return StatusTone.danger;
    return StatusTone.warning;
  }

  IconData _statusIcon() {
    switch (sub.status) {
      case SubscriptionStatus.active:
        return Icons.check_circle_outline;
      case SubscriptionStatus.suspended:
        return Icons.pause_circle_outline;
      case SubscriptionStatus.cancelled:
        return Icons.cancel_outlined;
      case SubscriptionStatus.pendingInstallation:
        return Icons.schedule;
      case SubscriptionStatus.grace:
        return Icons.hourglass_bottom;
      case SubscriptionStatus.expired:
        return Icons.timer_off_outlined;
      default:
        return Icons.help_outline;
    }
  }
}

// ── Speed section ─────────────────────────────────────────

class _SpeedSection extends StatelessWidget {
  const _SpeedSection({required this.sub, required this.l10n});
  final SubscriptionModel sub;
  final AppLocalizations l10n;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Column(
      children: [
        Row(
          children: [
            Expanded(
              child: IspStatCard(
                label: 'Download',
                value: sub.packageName ?? '-',
                helper: l10n.internetPackage ?? 'Paket',
                icon: Icons.arrow_downward_rounded,
                tone: StatusTone.info,
              ),
            ),
            const SizedBox(width: IspSpacing.md),
            Expanded(
              child: IspStatCard(
                label: 'Upload',
                value: sub.packageName ?? '-',
                helper: l10n.internetPackage ?? 'Paket',
                icon: Icons.arrow_upward_rounded,
                tone: StatusTone.primary,
              ),
            ),
          ],
        ),
        const SizedBox(height: IspSpacing.md),
        SizedBox(
          width: double.infinity,
          child: OutlinedButton.icon(
            onPressed: () => _openSpeedTest(context),
            icon: const Icon(Icons.speed, size: 18),
            label: Text(l10n.speedTest ?? 'Test Kecepatan'),
          ),
        ),
      ],
    );
  }

  void _openSpeedTest(BuildContext context) {
    final uri = Uri.parse('https://www.speedtest.net');
    canLaunchUrl(uri).then((can) {
      if (can) launchUrl(uri, mode: LaunchMode.externalApplication);
    });
  }
}

// ── Section card ──────────────────────────────────────────

class _SectionCard extends StatelessWidget {
  const _SectionCard({
    required this.title,
    required this.icon,
    required this.children,
  });

  final String title;
  final IconData icon;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return IspCard(
      child: Padding(
        padding: const EdgeInsets.all(IspSpacing.lg),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(icon, size: 18, color: isp.accent),
                const SizedBox(width: IspSpacing.sm),
                Text(
                  title,
                  style: TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: isp.textSecondary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: IspSpacing.md),
            ...children,
          ],
        ),
      ),
    );
  }
}

// ── Detail row (label + value) ────────────────────────────

class _DetailRow extends StatelessWidget {
  const _DetailRow({
    required this.icon,
    required this.label,
    required this.value,
  });

  final IconData icon;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Padding(
      padding: const EdgeInsets.symmetric(vertical: IspSpacing.sm),
      child: Row(
        children: [
          Icon(icon, size: 16, color: isp.textMuted),
          const SizedBox(width: IspSpacing.sm),
          SizedBox(
            width: 90,
            child: Text(
              label,
              style: TextStyle(
                fontSize: 13,
                color: isp.textMuted,
              ),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w500,
                color: isp.textPrimary,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// ── Info chip (used in hero) ──────────────────────────────

class _InfoChip extends StatelessWidget {
  const _InfoChip({required this.icon, required this.label});
  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: isp.surfaceTertiary,
        borderRadius: BorderRadius.circular(IspRadii.pill),
        border: Border.all(color: isp.borderSubtle),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: isp.accent),
          const SizedBox(width: 6),
          Text(
            label,
            style: TextStyle(
              color: isp.textPrimary,
              fontSize: 12,
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
      ),
    );
  }
}

// ── Installation tracker ────────────────────────────────────

class _InstallationTracker extends StatelessWidget {
  const _InstallationTracker({required this.sub, required this.l10n});
  final SubscriptionModel sub;
  final AppLocalizations l10n;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final steps = [
      _Step('Pendaftaran', Icons.app_registration, true),
      _Step('Penjadwalan', Icons.event, true),
      _Step('Pemasangan', Icons.build, false),
      _Step('Aktif', Icons.check_circle, false),
    ];

    return IspCard(
      child: Padding(
        padding: const EdgeInsets.all(IspSpacing.lg),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.construction,
                    size: 18, color: isp.accent),
                const SizedBox(width: IspSpacing.sm),
                Text(
                  'Progres Pemasangan',
                  style: TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: isp.textSecondary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: IspSpacing.lg),
            ...steps.asMap().entries.map((entry) {
              final i = entry.key;
              final step = entry.value;
              final isLast = i == steps.length - 1;
              return Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Column(
                    children: [
                      Container(
                        width: 28,
                        height: 28,
                        decoration: BoxDecoration(
                          shape: BoxShape.circle,
                          color:
                              step.done ? isp.success : isp.border,
                        ),
                        child: Icon(
                          step.done ? Icons.check : step.icon,
                          size: 16,
                          color:
                              step.done ? Colors.white : isp.textMuted,
                        ),
                      ),
                      if (!isLast)
                        Container(
                          width: 2,
                          height: 24,
                          color:
                              step.done ? isp.success : isp.border,
                        ),
                    ],
                  ),
                  const SizedBox(width: IspSpacing.md),
                  Padding(
                    padding: const EdgeInsets.only(top: 4),
                    child: Text(
                      step.label,
                      style: TextStyle(
                        fontSize: 14,
                        fontWeight:
                            step.done ? FontWeight.w600 : FontWeight.w400,
                        color: step.done
                            ? isp.textPrimary
                            : isp.textMuted,
                      ),
                    ),
                  ),
                ],
              );
            }),
          ],
        ),
      ),
    );
  }
}

class _Step {
  const _Step(this.label, this.icon, this.done);
  final String label;
  final IconData icon;
  final bool done;
}

// ── Circular progress painter (neubrutalist ring) ──
class _CircularProgressPainter extends CustomPainter {
  const _CircularProgressPainter({
    required this.progress,
    required this.color,
    required this.bgColor,
    required this.strokeWidth,
  });
  final double progress;
  final Color color;
  final Color bgColor;
  final double strokeWidth;

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = (size.width - strokeWidth) / 2;

    // Background ring
    final bgPaint = Paint()
      ..color = bgColor
      ..style = PaintingStyle.stroke
      ..strokeWidth = strokeWidth
      ..strokeCap = StrokeCap.round;
    canvas.drawCircle(center, radius, bgPaint);

    // Progress arc
    final progressPaint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = strokeWidth
      ..strokeCap = StrokeCap.round;
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: radius),
      -pi / 2, // start from top
      2 * pi * progress,
      false,
      progressPaint,
    );
  }

  @override
  bool shouldRepaint(covariant _CircularProgressPainter oldDelegate) =>
      oldDelegate.progress != progress || oldDelegate.color != color;
}
