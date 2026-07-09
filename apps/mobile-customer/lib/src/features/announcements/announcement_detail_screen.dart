import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/feature_providers.dart';

// ── Provider ──────────────────────────────────────────────

final announcementDetailProvider =
    FutureProvider.family<AnnouncementModel, String>((ref, id) async {
  final svc = ref.watch(announcementServiceProvider);
  final ServiceResult<AnnouncementModel> res = await svc.getById(id);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw exception.message,
  };
});

// ── Screen ────────────────────────────────────────────────

class AnnouncementDetailScreen extends ConsumerWidget {
  const AnnouncementDetailScreen({required this.id, super.key});
  final String id;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final dateFmt = DateFormat('d MMM yyyy, HH:mm', 'id_ID');
    final async = ref.watch(announcementDetailProvider(id));

    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.announcementDetail ?? 'Detail Pengumuman'),
      ),
      body: async.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.error_outline, size: 48, color: isp.danger),
              const SizedBox(height: IspSpacing.md),
              Text(e.toString(), textAlign: TextAlign.center),
              const SizedBox(height: IspSpacing.lg),
              OutlinedButton.icon(
                onPressed: () => ref.invalidate(announcementDetailProvider(id)),
                icon: const Icon(Icons.refresh),
                label: Text(l10n.retry ?? 'Coba Lagi'),
              ),
            ],
          ),
        ),
        data: (item) {
          final startDate =
              item.startsAt != null ? DateTime.tryParse(item.startsAt!) : null;
          final endDate =
              item.endsAt != null ? DateTime.tryParse(item.endsAt!) : null;

          return SingleChildScrollView(
            padding: const EdgeInsets.all(IspSpacing.lg),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                _Header(item: item),
                const SizedBox(height: IspSpacing.lg),
                Text(
                  item.plainBody,
                  style: TextStyle(
                    fontSize: 15,
                    height: 1.6,
                    color: isp.textPrimary,
                  ),
                ),
                const SizedBox(height: IspSpacing.xl),
                IspCard(
                  child: Padding(
                    padding: const EdgeInsets.all(IspSpacing.lg),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          l10n.details ?? 'Detail',
                          style: TextStyle(
                            fontSize: 14,
                            fontWeight: FontWeight.w600,
                            color: isp.textSecondary,
                          ),
                        ),
                        const SizedBox(height: IspSpacing.md),
                        if (startDate != null)
                          _DetailRow(
                            icon: Icons.event_outlined,
                            label: l10n.startsAt ?? 'Mulai',
                            value: dateFmt.format(startDate),
                          ),
                        if (endDate != null)
                          _DetailRow(
                            icon: Icons.event_busy_outlined,
                            label: l10n.endsAt ?? 'Berakhir',
                            value: dateFmt.format(endDate),
                          ),
                        _DetailRow(
                          icon: Icons.flag_outlined,
                          label: l10n.severity ?? 'Severity',
                          value: item.severityLabel,
                        ),
                        _DetailRow(
                          icon: Icons.group_outlined,
                          label: l10n.audience ?? 'Audience',
                          value: item.audience,
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}

class _Header extends StatelessWidget {
  const _Header({required this.item});
  final AnnouncementModel item;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final dateFmt = DateFormat('d MMM yyyy, HH:mm', 'id_ID');
    final date =
        item.createdAt != null ? DateTime.tryParse(item.createdAt!) : null;

    final Color color;
    final IconData icon;
    switch (item.severity) {
      case 'success':
        color = isp.success;
        icon = Icons.check_circle_outline;
        break;
      case 'warning':
        color = isp.warning;
        icon = Icons.warning_amber_outlined;
        break;
      case 'error':
        color = isp.danger;
        icon = Icons.error_outline;
        break;
      default:
        color = isp.info;
        icon = Icons.info_outline;
    }

    return Container(
      padding: const EdgeInsets.all(IspSpacing.xl),
      decoration: BoxDecoration(
        color: color.withOpacity(0.08),
        borderRadius: BorderRadius.circular(IspRadii.lg),
        border: Border.all(color: color.withOpacity(0.2)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(icon, color: color, size: 20),
              const SizedBox(width: IspSpacing.sm),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: color.withOpacity(0.15),
                  borderRadius: BorderRadius.circular(IspRadii.pill),
                ),
                child: Text(
                  item.severityLabel,
                  style: TextStyle(
                    fontSize: 12,
                    fontWeight: FontWeight.w600,
                    color: color,
                  ),
                ),
              ),
              const Spacer(),
              if (date != null)
                Text(
                  dateFmt.format(date),
                  style: TextStyle(
                    fontSize: 12,
                    color: isp.textMuted,
                  ),
                ),
            ],
          ),
          const SizedBox(height: IspSpacing.md),
          Text(
            item.title,
            style: const TextStyle(
              fontSize: 22,
              fontWeight: FontWeight.w700,
            ),
          ),
        ],
      ),
    );
  }
}

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
    final isp = context.isp;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          Icon(icon, size: 16, color: isp.textMuted),
          const SizedBox(width: IspSpacing.sm),
          SizedBox(
            width: 80,
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
              style: const TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w500,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
