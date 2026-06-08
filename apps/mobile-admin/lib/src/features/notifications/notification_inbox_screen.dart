import 'package:flutter/material.dart';

class NotificationInboxScreen extends StatelessWidget {
  const NotificationInboxScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Notifikasi'),
        actions: [
          TextButton(onPressed: () {}, child: const Text('Baca Semua')),
        ],
      ),
      body: ListView.builder(
        itemCount: 0,
        itemBuilder: (context, index) => ListTile(
          leading: CircleAvatar(
            backgroundColor: Theme.of(context).colorScheme.primary.withValues(alpha: 0.15),
            child: const Icon(Icons.notifications, size: 20),
          ),
          title: Text('Notifikasi #\$index'),
          subtitle: Text('2 jam lalu'),
        ),
      ),
    );
  }
}
