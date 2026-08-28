import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

class AnnouncementListScreen extends StatelessWidget {
  const AnnouncementListScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Pengumuman')),
      body: ListView.builder(
        itemCount: 0,
        itemBuilder: (context, index) => Card(
          margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
          child: ListTile(
            leading: CircleAvatar(
              backgroundColor: Colors.blue.withOpacity(0.15),
              child: const Icon(Icons.campaign, color: Colors.blue, size: 20),
            ),
            title: Text('Pengumuman #\$index'),
            subtitle: Text('Semua pelanggan • 2 hari lalu'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => context.go('/announcements/\$index'),
          ),
        ),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: () => context.go('/announcements/new'),
        child: const Icon(Icons.add),
      ),
    );
  }
}
