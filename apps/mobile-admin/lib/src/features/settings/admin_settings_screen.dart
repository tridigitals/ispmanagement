import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:mobile_admin/src/services/auth_providers.dart';

class AdminSettingsScreen extends ConsumerWidget {
  const AdminSettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Scaffold(
      appBar: AppBar(title: const Text('Pengaturan')),
      body: ListView(
        children: [
          const SizedBox(height: 8),
          _SectionHeader(title: 'Akun'),
          ListTile(
            leading: const CircleAvatar(child: Icon(Icons.person)),
            title: const Text('Profil Admin'),
            subtitle: const Text('Kelola informasi profil'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => context.go('/profile'),
          ),
          const Divider(height: 1),
          ListTile(
            leading: const Icon(Icons.notifications_outlined),
            title: const Text('Notifikasi'),
            subtitle: const Text('Pengaturan notifikasi push'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => context.go('/notifications'),
          ),
          const SizedBox(height: 16),
          _SectionHeader(title: 'Sistem'),
          ListTile(
            leading: const Icon(Icons.language),
            title: const Text('Bahasa'),
            subtitle: const Text('Indonesia'),
            trailing: const Icon(Icons.chevron_right),
          ),
          const Divider(height: 1),
          ListTile(
            leading: const Icon(Icons.palette_outlined),
            title: const Text('Tema'),
            subtitle: const Text('Ikuti sistem'),
            trailing: const Icon(Icons.chevron_right),
          ),
          const Divider(height: 1),
          ListTile(
            leading: const Icon(Icons.info_outline),
            title: const Text('Tentang'),
            subtitle: const Text('ISP Admin v0.1.0'),
          ),
          const SizedBox(height: 24),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: OutlinedButton.icon(
              onPressed: () {
                ref.read(authProvider.notifier).logout();
                context.go('/login');
              },
              icon: const Icon(Icons.logout, color: Colors.red),
              label: const Text('Keluar', style: TextStyle(color: Colors.red)),
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  const _SectionHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 4),
      child: Text(title.toUpperCase(), style: Theme.of(context).textTheme.bodySmall?.copyWith(
        fontWeight: FontWeight.w600,
        letterSpacing: 1.2,
      )),
    );
  }
}
