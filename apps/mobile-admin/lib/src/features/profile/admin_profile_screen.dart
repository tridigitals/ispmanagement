import 'package:flutter/material.dart';

class AdminProfileScreen extends StatelessWidget {
  const AdminProfileScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Profil Admin')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Center(
            child: Column(
              children: [
                const CircleAvatar(radius: 40, child: Icon(Icons.person, size: 40)),
                const SizedBox(height: 12),
                Text('Admin', style: Theme.of(context).textTheme.titleLarge),
                Text('admin@isp.com', style: Theme.of(context).textTheme.bodyMedium),
              ],
            ),
          ),
          const SizedBox(height: 24),
          Card(
            child: Column(
              children: [
                ListTile(leading: const Icon(Icons.badge), title: const Text('Nama'), subtitle: const Text('Admin')),
                const Divider(height: 1),
                ListTile(leading: const Icon(Icons.email), title: const Text('Email'), subtitle: const Text('admin@isp.com')),
                const Divider(height: 1),
                ListTile(leading: const Icon(Icons.phone), title: const Text('Telepon'), subtitle: const Text('—')),
                const Divider(height: 1),
                ListTile(leading: const Icon(Icons.shield), title: const Text('Role'), subtitle: const Text('Admin')),
              ],
            ),
          ),
          const SizedBox(height: 16),
          ElevatedButton.icon(
            onPressed: () {},
            icon: const Icon(Icons.edit),
            label: const Text('Edit Profil'),
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            onPressed: () {},
            icon: const Icon(Icons.lock),
            label: const Text('Ganti Password'),
          ),
        ],
      ),
    );
  }
}
