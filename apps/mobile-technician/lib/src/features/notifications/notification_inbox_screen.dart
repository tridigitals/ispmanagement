import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/notifications_providers.dart';
import '../../theme/app_theme.dart';
import '../../utils/loading_skeleton.dart';

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

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final l10n = AppLocalizations.of(context);
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
                    'Semua notifikasi akan dihapus. Tindakan ini tidak dapat dibatalkan.',
                  ),
                  actions: [
                    TextButton(
                      onPressed: () => Navigator.of(ctx).pop(false),
                      child: const Text('Batal'),
                    ),
                    TextButton(
                      onPressed: () => Navigator.of(ctx).pop(true),
                      style: TextButton.styleFrom(
                        foregroundColor: Colors.red,
                      ),
                      child: const Text('Hapus Semua'),
                    ),
                  ],
                ),
              );
              if (confirmed == true && context.mounted) {
                await ref.read(notificationsProvider.notifier).clearAll();
              }
            },
            child: const Text('Hapus Semua'),
          ),
          TextButton(
            onPressed: () async {
              await ref.read(notificationsProvider.notifier).markAllRead();
            },
            child: Text(l10n.markAllRead),
          ),
        ],
      ),
      body: async.when(
        loading: () => const IspSkeletonList(itemCount: 6),
        error: (e, _) => IspErrorState(
          message: e.toString(),
          onRetry: () => ref.invalidate(notificationsProvider),
        ),
        data: (list) {
          if (list.isEmpty) {
            return IspEmptyState(
              icon: Icons.notifications_off_outlined,
              title: l10n.noNotifications,
              message: 'Notifikasi akan muncul di sini',
            );
          }
          return NotificationListener<ScrollNotification>(
            onNotification: _onScroll,
            child: RefreshIndicator(
              color: isp.accent,
              onRefresh: () async {
                ref.invalidate(notificationsProvider);
                await ref.read(notificationsProvider.future);
              },
              child: ListView.builder(
                itemCount: list.length + 1,
                itemBuilder: (context, index) {
                  if (index == list.length) {
                    return _loadingMore
                        ? Padding(
                            padding: EdgeInsets.all(24),
                            child: Center(
                              child: SizedBox(
                                width: 24,
                                height: 24,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                  color: isp.accent,
                                ),
                              ),
                            ),
                          )
                        : const SizedBox.shrink();
                  }
                  return Dismissible(
                    key: Key(list[index].id),
                    direction: DismissDirection.endToStart,
                    background: Container(
                      color: Colors.red,
                      alignment: Alignment.centerRight,
                      padding: const EdgeInsets.only(right: 20),
                      child: const Icon(Icons.delete, color: Colors.white),
                    ),
                    onDismissed: (_) {
                      ref.read(notificationsProvider.notifier).delete(list[index].id);
                    },
                    child: _NotificationTile(item: list[index]),
                  );
                },
              ),
            ),
          );
        },
      ),
    );
  }
}

class _NotificationTile extends ConsumerWidget {
  const _NotificationTile({required this.item});
  final NotificationModel item;

  String? _resolveTarget(NotificationModel item) {
    // Priority 1: actionUrl from root payload (backend sends action_url)
    final actionUrl = item.actionUrl?.trim();
    if (actionUrl != null && actionUrl.isNotEmpty) {
      return _normalizeRoute(actionUrl);
    }
    // Priority 2: deepLink
    final deepLink = item.deepLink?.trim();
    if (deepLink != null && deepLink.isNotEmpty) {
      return _normalizeRoute(deepLink);
    }
    // Priority 3: action_url inside data map
    final action = item.data?['action_url']?.toString().trim();
    if (action != null && action.isNotEmpty) {
      return _normalizeRoute(action);
    }
    return null;
  }

  /// Map backend routes to mobile app routes.
  /// CRITICAL: Only return paths that match actual GoRouter routes!
  /// Available routes: /subscriptions/:id, /invoices/:id, /tickets/:id,
  ///   /payments/:invoiceId, /notifications, /profile, /settings, /?tab=N
  /// Tab mapping: 0=Home, 1=Subscriptions, 2=Invoices, 3=Support, 4=Profile
  String _normalizeRoute(String route) {
    // /support/{id} → /tickets/{id}
    if (route.startsWith('/support/')) {
      final id = route.substring('/support/'.length);
      if (id.isNotEmpty) return '/tickets/$id';
    }
    // /admin/support/{id} → /tickets/{id}
    if (route.startsWith('/admin/support/')) {
      final id = route.substring('/admin/support/'.length);
      if (id.isNotEmpty) return '/tickets/$id';
    }
    // /pay/{id} → /payments/{id}
    if (route.startsWith('/pay/')) {
      final id = route.substring('/pay/'.length);
      if (id.isNotEmpty) return '/payments/$id';
    }
    // /admin/invoices/{id} → /invoices/{id}
    if (route.startsWith('/admin/invoices/')) {
      final id = route.substring('/admin/invoices/'.length);
      if (id.isNotEmpty) return '/invoices/$id';
    }
    // /dashboard/invoices/{id} → /invoices/{id}
    if (route.startsWith('/dashboard/invoices/')) {
      final id = route.substring('/dashboard/invoices/'.length);
      if (id.isNotEmpty) return '/invoices/$id';
    }
    // /dashboard/invoices → Invoices tab
    if (route == '/dashboard/invoices') return '/?tab=2';
    // /dashboard/tickets/{id} → /tickets/{id}
    if (route.startsWith('/dashboard/tickets/')) {
      final id = route.substring('/dashboard/tickets/'.length);
      if (id.isNotEmpty) return '/tickets/$id';
    }
    // /dashboard/subscriptions/{id} → /subscriptions/{id}
    if (route.startsWith('/dashboard/subscriptions/')) {
      final id = route.substring('/dashboard/subscriptions/'.length);
      if (id.isNotEmpty) return '/subscriptions/$id';
    }
    // /dashboard/payments/{id} → /payments/{id}
    if (route.startsWith('/dashboard/payments/')) {
      final id = route.substring('/dashboard/payments/'.length);
      if (id.isNotEmpty) return '/payments/$id';
    }
    // /dashboard/services → Subscriptions tab
    if (route == '/dashboard/services' ||
        route.startsWith('/dashboard/services/')) {
      return '/?tab=1';
    }
    // /announcements/{id} → /announcements/{id}
    if (route.startsWith('/announcements/')) {
      final id = route.substring('/announcements/'.length);
      if (id.isNotEmpty) return '/announcements/$id';
    }
    // /announcements → /announcements (list)
    if (route == '/announcements') return '/announcements';
    // /admin/announcements/{id} → /announcements/{id}
    if (route.startsWith('/admin/announcements/')) {
      final id = route.substring('/admin/announcements/'.length);
      if (id.isNotEmpty) return '/announcements/$id';
    }
    // /admin/announcements → /announcements (list)
    if (route == '/admin/announcements') return '/announcements';
    // No matching route — fallback to home
    return '/?tab=0';
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    return Material(
      color: item.isUnread ? isp.surfaceElevated : Colors.transparent,
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
              // Route doesn't exist — fallback to home
              if (context.mounted) {
                GoRouter.of(context).go('/');
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: const Text(
                      'Halaman tidak tersedia di mobile',
                      style: TextStyle(color: Colors.white),
                    ),
                    backgroundColor: isp.warning,
                    duration: const Duration(seconds: 2),
                  ),
                );
              }
            }
          }
        },
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
          decoration: BoxDecoration(
            border: Border(
              bottom: BorderSide(color: isp.border, width: 0.5),
            ),
          ),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _IconFor(category: item.category),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      item.title,
                      style: TextStyle(
                        fontSize: 14,
                        fontWeight:
                            item.isUnread ? FontWeight.w600 : FontWeight.w500,
                        color: isp.textPrimary,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      item.body,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 13,
                        color: isp.textSecondary,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      item.categoryLabel,
                      style: TextStyle(
                        fontSize: 11,
                        color: isp.textMuted,
                      ),
                    ),
                  ],
                ),
              ),
              if (item.isUnread) ...[
                const SizedBox(width: 8),
                Padding(
                  padding: EdgeInsets.only(top: 6),
                  child: Icon(Icons.circle, color: isp.accent, size: 10),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _IconFor extends StatelessWidget {
  const _IconFor({required this.category});
  final NotificationCategory category;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final IconData icon;
    final Color color;
    switch (category) {
      case NotificationCategory.invoice:
        icon = Icons.receipt_long;
        color = isp.warning;
        break;
      case NotificationCategory.ticket:
        icon = Icons.support_agent;
        color = isp.info;
        break;
      case NotificationCategory.outage:
        icon = Icons.warning_amber;
        color = isp.danger;
        break;
      case NotificationCategory.payment:
        icon = Icons.payment;
        color = isp.success;
        break;
      case NotificationCategory.subscription:
        icon = Icons.wifi;
        color = isp.accent;
        break;
      case NotificationCategory.promo:
        icon = Icons.local_offer;
        color = isp.accent;
        break;
      case NotificationCategory.system:
        icon = Icons.info_outline;
        color = isp.textMuted;
        break;
    }
    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(10),
      ),
      child: Icon(icon, color: color, size: 20),
    );
  }
}
