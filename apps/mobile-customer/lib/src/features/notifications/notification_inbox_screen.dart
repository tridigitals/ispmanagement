import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/notifications_providers.dart';

// ─── Neubrutalist card ───────────────────────────────────────────

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

// ─── Time format ─────────────────────────────────────────────────

String _timeAgo(DateTime dt) {
  final diff = DateTime.now().difference(dt);
  if (diff.inMinutes < 60) return '${diff.inMinutes} menit yang lalu';
  if (diff.inHours < 24) return '${diff.inHours} jam yang lalu';
  return DateFormat('d MMM yyyy').format(dt);
}

String _dayGroup(DateTime dt) {
  final now = DateTime.now();
  final today = DateTime(now.year, now.month, now.day);
  final date = DateTime(dt.year, dt.month, dt.day);
  final diff = today.difference(date).inDays;
  if (diff == 0) return 'Hari ini';
  if (diff == 1) return 'Kemarin';
  return DateFormat('d MMMM yyyy').format(date);
}

class NotificationInboxScreen extends ConsumerStatefulWidget {
  const NotificationInboxScreen({super.key});

  @override
  ConsumerState<NotificationInboxScreen> createState() =>
      _NotificationInboxScreenState();
}

class _NotificationInboxScreenState
    extends ConsumerState<NotificationInboxScreen> {
  bool _loadingMore = false;

  Future<void> _loadMore() async {
    if (_loadingMore) return;
    final notifier = ref.read(notificationsProvider.notifier);
    if (!notifier.hasMore) return;
    setState(() => _loadingMore = true);
    await notifier.loadMore();
    if (mounted) setState(() => _loadingMore = false);
  }

  bool _onScroll(Notification notification) {
    if (notification is ScrollNotification &&
        notification.metrics.extentAfter <
            notification.metrics.maxScrollExtent * 0.1) {
      _loadMore();
    }
    return false;
  }

  /// Group notifications by day (Hari ini / Kemarin / date)
  List<MapEntry<String, List<NotificationModel>>> _grouped(
      List<NotificationModel> items) {
    final map = <String, List<NotificationModel>>{};
    for (final item in items) {
      final key = _dayGroup(item.createdAt);
      map.putIfAbsent(key, () => []).add(item);
    }
    return map.entries.toList();
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final async = ref.watch(notificationsProvider);

    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.notifications),
        actions: [
          TextButton(
            onPressed: () async {
              final confirmed = await showDialog<bool>(
                context: context,
                builder: (ctx) => AlertDialog(
                  title: const Text('Hapus Semua?'),
                  content: const Text(
                    'Semua notifikasi akan dihapus.\nTindakan ini tidak dapat dibatalkan.',
                  ),
                  actions: [
                    TextButton(
                      onPressed: () => Navigator.of(ctx).pop(false),
                      child: const Text('Batal'),
                    ),
                    TextButton(
                      onPressed: () => Navigator.of(ctx).pop(true),
                      style: TextButton.styleFrom(foregroundColor: isp.danger),
                      child: const Text('Hapus Semua'),
                    ),
                  ],
                ),
              );
              if (confirmed == true && context.mounted) {
                await ref.read(notificationsProvider.notifier).clearAll();
              }
            },
            child: Text('Hapus', style: TextStyle(fontSize: 13, color: isp.textMuted)),
          ),
        ],
      ),
      body: async.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Column(mainAxisSize: MainAxisSize.min, children: [
            Icon(Icons.error_outline, size: 48, color: isp.danger),
            const SizedBox(height: 12),
            Text('Gagal memuat', style: TextStyle(color: isp.textSecondary)),
            const SizedBox(height: 16),
            OutlinedButton(
              onPressed: () => ref.invalidate(notificationsProvider),
              child: Text(l10n.retry),
            ),
          ]),
        ),
        data: (items) {
          if (items.isEmpty) {
            return Center(
              child: Column(mainAxisSize: MainAxisSize.min, children: [
                Icon(Icons.notifications_none, size: 64, color: isp.textMuted),
                const SizedBox(height: 8),
                Text('Belum ada notifikasi', style: TextStyle(color: isp.textMuted)),
              ]),
            );
          }

          final groups = _grouped(items);

          return NotificationListener<ScrollNotification>(
            onNotification: _onScroll,
            child: ListView.builder(
              padding: const EdgeInsets.only(bottom: 48),
              itemCount: groups.length + (_loadingMore ? 1 : 0),
              itemBuilder: (context, index) {
                if (index >= groups.length) {
                  return const Padding(
                    padding: EdgeInsets.all(24),
                    child: Center(child: SizedBox(width: 24, height: 24, child: CircularProgressIndicator(strokeWidth: 2))),
                  );
                }
                final group = groups[index];
                return _DaySection(
                  label: group.key,
                  items: group.value,
                  isp: isp,
                );
              },
            ),
          );
        },
      ),
    );
  }
}

class _DaySection extends StatelessWidget {
  const _DaySection({
    required this.label,
    required this.items,
    required this.isp,
  });

  final String label;
  final List<NotificationModel> items;
  final IspThemeColors isp;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 12, 16, 8),
            child: Text(
              label.toUpperCase(),
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w700,
                color: isp.textMuted,
                letterSpacing: 1,
              ),
            ),
          ),
          ...items.map((item) => _NotificationTile(item: item)),
        ],
      ),
    );
  }
}

class _NotificationTile extends ConsumerWidget {
  const _NotificationTile({required this.item});
  final NotificationModel item;

  String? _resolveTarget(NotificationModel item) {
    final actionUrl = item.actionUrl?.trim();
    if (actionUrl != null && actionUrl.isNotEmpty) return _normalizeRoute(actionUrl);
    final deepLink = item.deepLink?.trim();
    if (deepLink != null && deepLink.isNotEmpty) return _normalizeRoute(deepLink);
    final action = item.data?['action_url']?.toString().trim();
    if (action != null && action.isNotEmpty) return _normalizeRoute(action);
    return null;
  }

  String _normalizeRoute(String route) {
    if (route.startsWith('/support/')) return '/tickets/${route.substring('/support/'.length)}';
    if (route.startsWith('/admin/support/')) return '/tickets/${route.substring('/admin/support/'.length)}';
    if (route.startsWith('/pay/')) return '/payments/${route.substring('/pay/'.length)}';
    if (route.startsWith('/admin/invoices/')) return '/invoices/${route.substring('/admin/invoices/'.length)}';
    if (route.startsWith('/dashboard/invoices/')) return '/invoices/${route.substring('/dashboard/invoices/'.length)}';
    if (route == '/dashboard/invoices') return '/?tab=2';
    if (route.startsWith('/dashboard/tickets/')) return '/tickets/${route.substring('/dashboard/tickets/'.length)}';
    if (route.startsWith('/dashboard/subscriptions/')) return '/subscriptions/${route.substring('/dashboard/subscriptions/'.length)}';
    if (route.startsWith('/dashboard/payments/')) return '/payments/${route.substring('/dashboard/payments/'.length)}';
    if (route == '/dashboard/services' || route.startsWith('/dashboard/services/')) return '/?tab=1';
    if (route.startsWith('/announcements/')) return '/announcements/${route.substring('/announcements/'.length)}';
    if (route == '/announcements') return '/announcements';
    if (route.startsWith('/admin/announcements/')) return '/announcements/${route.substring('/admin/announcements/'.length)}';
    if (route == '/admin/announcements') return '/announcements';
    return '/?tab=0';
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 3),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () async {
            if (item.isUnread) {
              await ref.read(notificationsProvider.notifier).markRead(item.id);
            }
            final target = _resolveTarget(item);
            if (target != null && context.mounted) {
              try {
                GoRouter.of(context).push(target);
              } catch (_) {
                if (context.mounted) GoRouter.of(context).go('/');
              }
            }
          },
          borderRadius: BorderRadius.circular(16),
          child: Container(
            decoration: _nbCard(isp),
            padding: const EdgeInsets.all(12),
            child: Row(children: [
              // Tinted icon circle (36px, match mockup ico-r)
              Container(
                width: 36, height: 36,
                decoration: BoxDecoration(
                  color: _colorFor(item.category, isp).withOpacity(0.1),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(_iconFor(item.category), size: 16, color: _colorFor(item.category, isp)),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text(
                    item.title,
                    style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: isp.textPrimary),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 2),
                  Text(_timeAgo(item.createdAt), style: TextStyle(fontSize: 10, color: isp.textMuted)),
                ]),
              ),
              if (item.isUnread)
                Container(
                  width: 8, height: 8,
                  margin: const EdgeInsets.only(left: 8),
                  decoration: BoxDecoration(color: isp.accent, shape: BoxShape.circle),
                ),
            ]),
          ),
        ),
      ),
    );
  }
}

IconData _iconFor(NotificationCategory c) {
  switch (c) {
    case NotificationCategory.invoice:
      return Icons.receipt_long;
    case NotificationCategory.ticket:
      return Icons.support_agent;
    case NotificationCategory.outage:
      return Icons.warning_amber;
    case NotificationCategory.payment:
      return Icons.check_circle;
    case NotificationCategory.subscription:
      return Icons.wifi;
    case NotificationCategory.promo:
      return Icons.local_offer;
    case NotificationCategory.system:
      return Icons.info_outline;
  }
}

Color _colorFor(NotificationCategory c, IspThemeColors isp) {
  switch (c) {
    case NotificationCategory.invoice:
      return isp.danger;
    case NotificationCategory.ticket:
      return isp.info;
    case NotificationCategory.outage:
      return isp.danger;
    case NotificationCategory.payment:
      return isp.success;
    case NotificationCategory.subscription:
      return isp.accent;
    case NotificationCategory.promo:
      return isp.accent;
    case NotificationCategory.system:
      return isp.textMuted;
  }
}
