import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../../l10n/app_localizations.dart';
import '../../../services/feature_providers.dart';
import '../../../services/notifications_providers.dart';

class NotificationInboxScreen extends ConsumerWidget {
  const NotificationInboxScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context)!;
    final async = ref.watch(notificationsProvider);

    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.notifications),
        actions: [
          TextButton(
            onPressed: () async {
              await ref.read(notificationsProvider.notifier).markAllRead();
            },
            child: Text(l10n.markAllRead),
          ),
        ],
      ),
      body: async.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text(e.toString())),
        data: (list) {
          if (list.isEmpty) {
            return Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.notifications_off_outlined, size: 48),
                  const SizedBox(height: 12),
                  Text(l10n.noNotifications),
                ],
              ),
            );
          }
          return RefreshIndicator(
            onRefresh: () async {
              ref.invalidate(notificationsProvider);
              await ref.read(notificationsProvider.future);
            },
            child: ListView.separated(
              itemBuilder: (_, i) => _NotificationTile(item: list[i]),
              separatorBuilder: (_, __) =>
                  const Divider(height: 1, color: IspColors.borderSubtle),
              itemCount: list.length,
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

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      color: item.isUnread ? IspColors.primarySubtle : null,
      child: ListTile(
        leading: _IconFor(category: item.category),
        title: Text(
          item.title,
          style: TextStyle(
            fontWeight: item.isUnread ? FontWeight.w600 : FontWeight.w500,
          ),
        ),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const SizedBox(height: 4),
            Text(item.body, maxLines: 2, overflow: TextOverflow.ellipsis),
            const SizedBox(height: 4),
            Text(
              item.categoryLabel,
              style: const TextStyle(
                fontSize: 11,
                color: IspColors.textTertiary,
              ),
            ),
          ],
        ),
        trailing: item.isUnread
            ? const Icon(Icons.circle, color: IspColors.primary, size: 10)
            : null,
        onTap: () async {
          if (item.isUnread) {
            await ref
                .read(notificationsProvider.notifier)
                .markRead(item.id);
          }
          if (item.deepLink != null && context.mounted) {
            context.push(item.deepLink!);
          }
        },
      ),
    );
  }
}

class _IconFor extends StatelessWidget {
  const _IconFor({required this.category});
  final NotificationCategory category;

  @override
  Widget build(BuildContext context) {
    final IconData icon;
    final Color color;
    switch (category) {
      case NotificationCategory.invoice:
        icon = Icons.receipt_long;
        color = IspColors.warning;
        break;
      case NotificationCategory.ticket:
        icon = Icons.support_agent;
        color = IspColors.info;
        break;
      case NotificationCategory.outage:
        icon = Icons.warning_amber;
        color = IspColors.danger;
        break;
      case NotificationCategory.payment:
        icon = Icons.payment;
        color = IspColors.success;
        break;
      case NotificationCategory.subscription:
        icon = Icons.wifi;
        color = IspColors.primary;
        break;
      case NotificationCategory.promo:
        icon = Icons.local_offer;
        color = IspColors.primary;
        break;
      case NotificationCategory.system:
        icon = Icons.info_outline;
        color = IspColors.textTertiary;
        break;
    }
    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(IspRadii.sm),
      ),
      child: Icon(icon, color: color, size: 20),
    );
  }
}
