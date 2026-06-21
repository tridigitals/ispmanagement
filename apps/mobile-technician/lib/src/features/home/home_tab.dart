import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../services/auth_providers.dart';
import '../../services/settings_providers.dart';
import '../../services/ticket_providers.dart';
import '../tickets/widgets/ticket_card.dart';

/// Home tab — dashboard with stats cards + recent tickets.
class HomeTab extends ConsumerWidget {
  const HomeTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final user = ref.watch(currentUserProvider);
    final isp = context.isp;
    final stats = ref.watch(ticketStatsProvider);
    final tickets = ref.watch(todaysTicketsProvider);

    return RefreshIndicator(
      onRefresh: () async {
        ref.invalidate(myTicketsProvider);
        ref.invalidate(ticketStatsProvider);
        await Future.delayed(const Duration(milliseconds: 500));
      },
      child: ListView(
        padding: const EdgeInsets.symmetric(vertical: 8),
        children: [
          // Greeting
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Berikut daftar tiket yang ditugaskan kepadamu.',
                  style: TextStyle(
                    color: isp.textMuted,
                    fontSize: 14,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),

          // Stats cards
          stats.when(
            data: (data) => _StatsRow(stats: data),
            loading: () => const Padding(
              padding: EdgeInsets.symmetric(vertical: 24),
              child: Center(child: CircularProgressIndicator()),
            ),
            error: (e, _) => Padding(
              padding: const EdgeInsets.all(16),
              child: Card(
                color: isp.danger.withOpacity(0.1),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Text('Gagal memuat statistik: $e'),
                ),
              ),
            ),
          ),
          const SizedBox(height: 16),

          // Recent tickets
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: [
                Text(
                  'Tiket Aktif (${tickets.length})',
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                    color: isp.textPrimary,
                  ),
                ),
                const Spacer(),
                TextButton(
                  onPressed: () =>
                      ref.read(currentTabProvider.notifier).state = 1,
                  child: const Text('Lihat semua'),
                ),
              ],
            ),
          ),
          if (tickets.isEmpty)
            Padding(
              padding: const EdgeInsets.all(32),
              child: Center(
                child: Column(
                  children: [
                    Icon(Icons.inbox_outlined,
                        size: 48, color: isp.textMuted),
                    const SizedBox(height: 12),
                    Text(
                      'Tidak ada tiket aktif saat ini.',
                      style: TextStyle(
                        color: isp.textMuted,
                        fontSize: 14,
                      ),
                    ),
                  ],
                ),
              ),
            )
          else
            ...tickets.take(5).map((t) => TicketCard(
                  ticket: t,
                  onTap: () => context.push('/tickets/${t.id}'),
                )),
          const SizedBox(height: 80),
        ],
      ),
    );
  }
}

class _StatsRow extends StatelessWidget {
  const _StatsRow({required this.stats});
  final TicketStats stats;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      child: Row(
        children: [
          Expanded(
              child: _StatCard(
            label: 'Total',
            value: stats.all,
            color: Colors.blue,
            icon: Icons.list_alt,
          )),
          const SizedBox(width: 8),
          Expanded(
              child: _StatCard(
            label: 'Open',
            value: stats.open,
            color: Colors.orange,
            icon: Icons.error_outline,
          )),
          const SizedBox(width: 8),
          Expanded(
              child: _StatCard(
            label: 'Selesai',
            value: stats.closed,
            color: Colors.green,
            icon: Icons.check_circle_outline,
          )),
        ],
      ),
    );
  }
}

class _StatCard extends StatelessWidget {
  const _StatCard({
    required this.label,
    required this.value,
    required this.color,
    required this.icon,
  });
  final String label;
  final int value;
  final Color color;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Card(
      elevation: 0,
      color: color.withOpacity(0.08),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: color.withOpacity(0.3)),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(icon, color: color, size: 20),
            const SizedBox(height: 8),
            Text(
              '$value',
              style: TextStyle(
                fontSize: 24,
                fontWeight: FontWeight.bold,
                color: color,
              ),
            ),
            Text(
              label,
              style: TextStyle(
                fontSize: 12,
                color: color.withOpacity(0.8),
                fontWeight: FontWeight.w500,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
