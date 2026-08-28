import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

class DashboardScreen extends ConsumerWidget {
  const DashboardScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Dashboard'),
        actions: [
          IconButton(icon: const Icon(Icons.notifications_outlined), onPressed: () {}),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: () async {},
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            // Stats grid
            GridView.count(
              crossAxisCount: 2,
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              crossAxisSpacing: 12,
              mainAxisSpacing: 12,
              childAspectRatio: 1.6,
              children: [
                _StatCard(icon: Icons.people, label: 'Pelanggan', value: '—', color: Colors.blue),
                _StatCard(icon: Icons.receipt, label: 'Invoice Aktif', value: '—', color: Colors.orange),
                _StatCard(icon: Icons.support_agent, label: 'Tiket Terbuka', value: '—', color: Colors.red),
                _StatCard(icon: Icons.payments, label: 'Revenue', value: '—', color: Colors.green),
              ],
            ),
            const SizedBox(height: 24),

            // Recent tickets
            Text('Tiket Terbaru', style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            Card(
              child: ListView.separated(
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                itemCount: 3,
                separatorBuilder: (_, __) => const Divider(height: 1),
                itemBuilder: (context, index) => ListTile(
                  leading: CircleAvatar(
                    backgroundColor: Colors.orange.withOpacity(0.15),
                    child: const Icon(Icons.support_agent, color: Colors.orange, size: 20),
                  ),
                  title: Text('Tiket #${index + 1}'),
                  subtitle: Text('Menunggu respon', style: theme.textTheme.bodySmall),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () {},
                ),
              ),
            ),
            const SizedBox(height: 24),

            // Recent payments
            Text('Pembayaran Terbaru', style: theme.textTheme.titleMedium),
            const SizedBox(height: 12),
            Card(
              child: ListView.separated(
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                itemCount: 3,
                separatorBuilder: (_, __) => const Divider(height: 1),
                itemBuilder: (context, index) => ListTile(
                  leading: CircleAvatar(
                    backgroundColor: Colors.green.withOpacity(0.15),
                    child: const Icon(Icons.check_circle, color: Colors.green, size: 20),
                  ),
                  title: Text('Pembayaran #${index + 1}'),
                  subtitle: Text('Rp ${(index + 1) * 150}.000', style: theme.textTheme.bodySmall),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () {},
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _StatCard extends StatelessWidget {
  final IconData icon;
  final String label;
  final String value;
  final Color color;

  const _StatCard({required this.icon, required this.label, required this.value, required this.color});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(icon, color: color, size: 24),
            const SizedBox(height: 8),
            Text(value, style: theme.textTheme.headlineMedium?.copyWith(fontWeight: FontWeight.w700)),
            Text(label, style: theme.textTheme.bodySmall),
          ],
        ),
      ),
    );
  }
}
