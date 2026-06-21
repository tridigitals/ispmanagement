import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:ui_kit/ui_kit.dart';
import '../../services/notifications_providers.dart';

class NotificationInboxScreen extends ConsumerWidget {
  const NotificationInboxScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final notifs = ref.watch(notificationsProvider);

    return Scaffold(
      appBar: AppBar(title: const Text('Notifikasi')),
      body: notifs.when(
        data: (list) {
          if (list.isEmpty) {
            return Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.notifications_none, size: 56, color: isp.textMuted),
                  const SizedBox(height: 12),
                  Text('Belum ada notifikasi', style: TextStyle(color: isp.textMuted)),
                ],
              ),
            );
          }
          return ListView.builder(
            itemCount: list.length,
            itemBuilder: (_, i) {
              final n = list[i];
              return ListTile(
                leading: Icon(Icons.circle_notifications, color: isp.accent),
                title: Text(n.title),
                subtitle: Text(n.body, maxLines: 2, overflow: TextOverflow.ellipsis),
              );
            },
          );
        },
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('Error: $e')),
      ),
    );
  }
}
