import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/feature_providers.dart';

// ── Provider ──────────────────────────────────────────────

final announcementsListProvider = FutureProvider.autoDispose
    .family<PaginatedResponse<AnnouncementModel>, int>((ref, page) async {
  final svc = ref.watch(announcementServiceProvider);
  final ServiceResult<PaginatedResponse<AnnouncementModel>> res =
      await svc.listRecent(page: page, perPage: 20);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw exception.message,
  };
});

// ── Screen ────────────────────────────────────────────────

class AnnouncementsScreen extends ConsumerWidget {
  const AnnouncementsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final announcements = ref.watch(announcementsListProvider(1));

    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.announcements ?? 'Pengumuman'),
      ),
      body: announcements.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.error_outline,
                  size: 48, color: IspColors.danger),
              const SizedBox(height: IspSpacing.md),
              Text(e.toString(), textAlign: TextAlign.center),
              const SizedBox(height: IspSpacing.lg),
              OutlinedButton.icon(
                onPressed: () => ref.invalidate(announcementsListProvider(1)),
                icon: const Icon(Icons.refresh),
                label: Text(l10n.retry ?? 'Coba Lagi'),
              ),
            ],
          ),
        ),
        data: (data) {
          final items = data.data;
          if (items.isEmpty) {
            return Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(
                    Icons.campaign_outlined,
                    size: 64,
                    color: IspColors.textTertiary,
                  ),
                  const SizedBox(height: IspSpacing.md),
                  Text(
                    l10n.noAnnouncements ?? 'Belum ada pengumuman',
                    style: const TextStyle(
                      fontSize: 16,
                      color: IspColors.textSecondary,
                    ),
                  ),
                ],
              ),
            );
          }
          return RefreshIndicator(
            onRefresh: () async {
              ref.invalidate(announcementsListProvider(1));
            },
            child: ListView.separated(
              padding: const EdgeInsets.all(IspSpacing.lg),
              itemCount: items.length,
              separatorBuilder: (_, __) =>
                  const SizedBox(height: IspSpacing.md),
              itemBuilder: (_, i) => _AnnouncementCard(item: items[i]),
            ),
          );
        },
      ),
    );
  }
}

// ── Card ──────────────────────────────────────────────────

class _AnnouncementCard extends StatelessWidget {
  const _AnnouncementCard({required this.item});
  final AnnouncementModel item;

  @override
  Widget build(BuildContext context) {
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');
    final date =
        item.createdAt != null ? DateTime.tryParse(item.createdAt!) : null;

    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: () => GoRouter.of(context).push('/announcements/${item.id}'),
        child: Padding(
          padding: const EdgeInsets.all(IspSpacing.lg),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  _SeverityChip(severity: item.severity),
                  const Spacer(),
                  if (date != null)
                    Text(
                      dateFmt.format(date),
                      style: const TextStyle(
                        fontSize: 12,
                        color: IspColors.textTertiary,
                      ),
                    ),
                ],
              ),
              const SizedBox(height: IspSpacing.sm),
              Text(
                item.title,
                style: const TextStyle(
                  fontSize: 16,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: IspSpacing.xs),
              Text(
                item.plainBody,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  fontSize: 14,
                  color: IspColors.textSecondary,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SeverityChip extends StatelessWidget {
  const _SeverityChip({required this.severity});
  final String severity;

  @override
  Widget build(BuildContext context) {
    final Color color;
    final IconData icon;
    switch (severity) {
      case 'success':
        color = IspColors.success;
        icon = Icons.check_circle_outline;
        break;
      case 'warning':
        color = IspColors.warning;
        icon = Icons.warning_amber_outlined;
        break;
      case 'error':
        color = IspColors.danger;
        icon = Icons.error_outline;
        break;
      default:
        color = IspColors.info;
        icon = Icons.info_outline;
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.1),
        borderRadius: BorderRadius.circular(IspRadii.pill),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: color),
          const SizedBox(width: 4),
          Text(
            severity[0].toUpperCase() + severity.substring(1),
            style: TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.w600,
              color: color,
            ),
          ),
        ],
      ),
    );
  }
}
