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
    final l10n = AppLocalizations.of(context);
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');
    final subAsync = ref.watch(subscriptionByIdProvider(id));

    return Scaffold(
      appBar: AppBar(
          title: Text(l10n?.subscriptionDetail ?? 'Detail Langganan')),
      body: subAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Padding(
            padding: const EdgeInsets.all(IspSpacing.xl),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.error_outline,
                    size: 48, color: IspColors.danger),
                const SizedBox(height: IspSpacing.md),
                Text(e.toString(), textAlign: TextAlign.center),
                const SizedBox(height: IspSpacing.lg),
                OutlinedButton.icon(
                  onPressed: () =>
                      ref.invalidate(subscriptionByIdProvider(id)),
                  icon: const Icon(Icons.refresh),
                  label: Text(l10n?.retry ?? 'Coba Lagi'),
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
                title: l10n?.connectionDetails ?? 'Detail Koneksi',
                icon: Icons.settings_ethernet,
                children: [
                  _DetailRow(
                    icon: Icons.router,
                    label: l10n?.router ?? 'Router',
                    value: sub.routerName ?? '-',
                  ),
                  _DetailRow(
                    icon: Icons.location_on_outlined,
                    label: l10n?.location ?? 'Lokasi',
                    value: sub.locationLabel ?? '-',
                  ),
                  if (sub.notes != null && sub.notes!.isNotEmpty)
                    _DetailRow(
                      icon: Icons.note_outlined,
                      label: l10n?.notes ?? 'Catatan',
                      value: sub.notes!,
                    ),
                ],
              ),
              const SizedBox(height: IspSpacing.lg),

              // ── Billing info ─────────────────────────────
              _SectionCard(
                title: l10n?.billingInfo ?? 'Informasi Tagihan',
                icon: Icons.receipt_long_outlined,
                children: [
                  _DetailRow(
                    icon: Icons.payments_outlined,
                    label: l10n?.price ?? 'Harga',
                    value:
                        '${fmt.format(sub.price)} / ${sub.billingCycle}',
                  ),
                  _DetailRow(
                    icon: Icons.calendar_today_outlined,
                    label: l10n?.cycle ?? 'Siklus',
                    value: _billingCycleLabel(sub.billingCycle),
                  ),
                  if (sub.startsAt != null)
                    _DetailRow(
                      icon: Icons.event_available_outlined,
                      label: l10n?.startsAt ?? 'Mulai',
                      value: dateFmt.format(sub.startsAt!),
                    ),
                  if (sub.endsAt != null)
                    _DetailRow(
                      icon: Icons.event_busy_outlined,
                      label: l10n?.endsAt ?? 'Berakhir',
                      value: dateFmt.format(sub.endsAt!),
                    ),
                  if (sub.graceUntil != null)
                    _DetailRow(
                      icon: Icons.hourglass_bottom_outlined,
                      label: l10n?.gracePeriod ?? 'Masa tenggang',
                      value: dateFmt.format(sub.graceUntil!),
                    ),
                ],
              ),
              const SizedBox(height: IspSpacing.xl),

              // ── Actions ──────────────────────────────────
              if (sub.status == SubscriptionStatus.pendingInstallation) ...[
                _InstallationTracker(sub: sub, l10n: l10n),
                const SizedBox(height: IspSpacing.lg),
              ],
              Row(
                children: [
                  Expanded(
                    child: OutlinedButton.icon(
                      onPressed: () => context.push('/tickets/new'),
                      icon: const Icon(Icons.report_problem_outlined),
                      label: Text(l10n?.reportOutage ?? 'Lapor Gangguan'),
                    ),
                  ),
                  const SizedBox(width: IspSpacing.md),
                  Expanded(
                    child: OutlinedButton.icon(
                      onPressed: () => _showUpgradeSheet(context, ref),
                      icon: const Icon(Icons.upgrade),
                      label: Text('Ubah Paket'),
                    ),
                  ),
                ],
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

  void _showUpgradeSheet(BuildContext context, WidgetRef ref) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) => DraggableScrollableSheet(
        initialChildSize: 0.6,
        maxChildSize: 0.9,
        minChildSize: 0.3,
        expand: false,
        builder: (ctx, scrollCtrl) => _PackageSheet(scrollCtrl: scrollCtrl),
      ),
    );
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
  final AppLocalizations? l10n;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(IspSpacing.xl),
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            _gradientStart(),
            _gradientEnd(),
          ],
        ),
        borderRadius: BorderRadius.circular(IspRadii.xl),
        boxShadow: IspShadows.md,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Package name + status
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text(
                  sub.packageName ??
                      (l10n?.internetPackage ?? 'Paket Internet'),
                  style: const TextStyle(
                    color: Colors.white70,
                    fontSize: 14,
                    fontWeight: FontWeight.w500,
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
          const SizedBox(height: IspSpacing.xl),
          // Price
          Text(
            fmt.format(sub.price),
            style: const TextStyle(
              color: Colors.white,
              fontSize: 36,
              fontWeight: FontWeight.w800,
              height: 1.0,
            ),
          ),
          const SizedBox(height: 4),
          Text(
            '/ ${sub.billingCycle}',
            style:
                const TextStyle(color: Colors.white60, fontSize: 14),
          ),
          const SizedBox(height: IspSpacing.lg),
          // Router + Location chips
          Wrap(
            spacing: IspSpacing.sm,
            runSpacing: IspSpacing.xs,
            children: [
              if (sub.routerName != null)
                _InfoChip(
                    icon: Icons.router, label: sub.routerName!),
              if (sub.locationLabel != null)
                _InfoChip(
                    icon: Icons.location_on_outlined,
                    label: sub.locationLabel!),
            ],
          ),
        ],
      ),
    );
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

  Color _gradientStart() {
    switch (sub.status) {
      case SubscriptionStatus.active:
        return const Color(0xFF8B9CFF);
      case SubscriptionStatus.suspended:
        return const Color(0xFFF59E0B);
      case SubscriptionStatus.cancelled:
        return const Color(0xFFEF4444);
      case SubscriptionStatus.grace:
        return const Color(0xFFF59E0B);
      default:
        return const Color(0xFF6B7280);
    }
  }

  Color _gradientEnd() {
    switch (sub.status) {
      case SubscriptionStatus.active:
        return const Color(0xFF6677EE);
      case SubscriptionStatus.suspended:
        return const Color(0xFFD97706);
      case SubscriptionStatus.cancelled:
        return const Color(0xFFDC2626);
      case SubscriptionStatus.grace:
        return const Color(0xFFD97706);
      default:
        return const Color(0xFF4B5563);
    }
  }
}

// ── Speed section ─────────────────────────────────────────

class _SpeedSection extends StatelessWidget {
  const _SpeedSection({required this.sub, required this.l10n});
  final SubscriptionModel sub;
  final AppLocalizations? l10n;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Row(
          children: [
            Expanded(
              child: IspStatCard(
                label: 'Download',
                value: sub.packageName ?? '-',
                helper: l10n?.internetPackage ?? 'Paket',
                icon: Icons.arrow_downward_rounded,
                tone: StatusTone.info,
              ),
            ),
            const SizedBox(width: IspSpacing.md),
            Expanded(
              child: IspStatCard(
                label: 'Upload',
                value: sub.packageName ?? '-',
                helper: l10n?.internetPackage ?? 'Paket',
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
            label: Text(l10n?.speedTest ?? 'Test Kecepatan'),
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
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(IspSpacing.lg),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(icon, size: 18, color: IspColors.primary),
                const SizedBox(width: IspSpacing.sm),
                Text(
                  title,
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: IspColors.textSecondary,
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
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: IspSpacing.sm),
      child: Row(
        children: [
          Icon(icon, size: 16, color: IspColors.textTertiary),
          const SizedBox(width: IspSpacing.sm),
          SizedBox(
            width: 90,
            child: Text(
              label,
              style: const TextStyle(
                fontSize: 13,
                color: IspColors.textTertiary,
              ),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: const TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w500,
                color: IspColors.textPrimary,
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
    return Container(
      padding:
          const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: Colors.white.withOpacity(0.12),
        borderRadius: BorderRadius.circular(IspRadii.pill),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: Colors.white70),
          const SizedBox(width: 6),
          Text(
            label,
            style: const TextStyle(
              color: Colors.white,
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
  final AppLocalizations? l10n;

  @override
  Widget build(BuildContext context) {
    final steps = [
      _Step('Pendaftaran', Icons.app_registration, true),
      _Step('Penjadwalan', Icons.event, true),
      _Step('Pemasangan', Icons.build, false),
      _Step('Aktif', Icons.check_circle, false),
    ];

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(IspSpacing.lg),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.construction, size: 18, color: IspColors.primary),
                const SizedBox(width: IspSpacing.sm),
                Text(
                  'Progres Pemasangan',
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: IspColors.textSecondary,
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
                          color: step.done
                              ? IspColors.success
                              : IspColors.border,
                        ),
                        child: Icon(
                          step.done ? Icons.check : step.icon,
                          size: 16,
                          color: step.done ? Colors.white : IspColors.textTertiary,
                        ),
                      ),
                      if (!isLast)
                        Container(
                          width: 2,
                          height: 24,
                          color: step.done ? IspColors.success : IspColors.border,
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
                        fontWeight: step.done ? FontWeight.w600 : FontWeight.w400,
                        color: step.done ? IspColors.textPrimary : IspColors.textTertiary,
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

// ── Package upgrade sheet ───────────────────────────────────

class _PackageSheet extends ConsumerWidget {
  const _PackageSheet({required this.scrollCtrl});
  final ScrollController scrollCtrl;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');

    return Column(
      children: [
        // Handle bar
        Container(
          margin: const EdgeInsets.only(top: 12, bottom: 8),
          width: 40,
          height: 4,
          decoration: BoxDecoration(
            color: IspColors.border,
            borderRadius: BorderRadius.circular(2),
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
          child: Text(
            'Paket Tersedia',
            style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
          ),
        ),
        Expanded(
          child: ListView(
            controller: scrollCtrl,
            padding: const EdgeInsets.symmetric(horizontal: 20),
            children: [
              _PackageCard(
                name: 'Paket Basic',
                speed: '10 Mbps',
                price: 150000,
                fmt: fmt,
                onTap: () => Navigator.pop(context),
              ),
              _PackageCard(
                name: 'Paket Standard',
                speed: '20 Mbps',
                price: 250000,
                fmt: fmt,
                onTap: () => Navigator.pop(context),
              ),
              _PackageCard(
                name: 'Paket Premium',
                speed: '50 Mbps',
                price: 450000,
                fmt: fmt,
                onTap: () => Navigator.pop(context),
              ),
              _PackageCard(
                name: 'Paket Ultimate',
                speed: '100 Mbps',
                price: 750000,
                fmt: fmt,
                onTap: () => Navigator.pop(context),
              ),
              const SizedBox(height: 20),
              Text(
                'Hubungi admin untuk mengubah paket',
                textAlign: TextAlign.center,
                style: const TextStyle(
                  color: IspColors.textTertiary,
                  fontSize: 13,
                ),
              ),
              const SizedBox(height: 40),
            ],
          ),
        ),
      ],
    );
  }
}

class _PackageCard extends StatelessWidget {
  const _PackageCard({
    required this.name,
    required this.speed,
    required this.price,
    required this.fmt,
    required this.onTap,
  });

  final String name;
  final String speed;
  final double price;
  final NumberFormat fmt;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: ListTile(
        leading: Container(
          width: 48,
          height: 48,
          decoration: BoxDecoration(
            color: IspColors.primary.withOpacity(0.1),
            borderRadius: BorderRadius.circular(12),
          ),
          child: const Icon(Icons.wifi, color: IspColors.primary),
        ),
        title: Text(name, style: const TextStyle(fontWeight: FontWeight.w600)),
        subtitle: Text(speed),
        trailing: Text(
          fmt.format(price),
          style: const TextStyle(fontWeight: FontWeight.w700, fontSize: 14),
        ),
        onTap: onTap,
      ),
    );
  }
}
