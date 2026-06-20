import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:api_client/api_client.dart';

import '../../services/ticket_providers.dart';
import 'widgets/ticket_card.dart';

/// Full list of tickets assigned to the technician.
/// Filter chips: Semua | Open | Diproses | Selesai
class TicketsScreen extends ConsumerStatefulWidget {
  const TicketsScreen({super.key});

  @override
  ConsumerState<TicketsScreen> createState() => _TicketsScreenState();
}

class _TicketsScreenState extends ConsumerState<TicketsScreen> {
  String? _filter; // null = all

  static const _filters = [
    (null, 'Semua'),
    ('open', 'Open'),
    ('inProgress', 'Diproses'),
    ('resolved', 'Selesai'),
  ];

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final state = ref.watch(myTicketsProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Daftar Tiket'),
        backgroundColor: theme.colorScheme.primary,
        foregroundColor: theme.colorScheme.onPrimary,
      ),
      body: Column(
        children: [
          // Filter chips
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            child: Row(
              children: _filters.map((f) {
                final selected = _filter == f.$1;
                return Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 4),
                  child: ChoiceChip(
                    label: Text(f.$2),
                    selected: selected,
                    onSelected: (_) => setState(() => _filter = f.$1),
                  ),
                );
              }).toList(),
            ),
          ),
          const Divider(height: 1),
          Expanded(
            child: state.when(
              data: (paginated) {
                final filtered = _applyFilter(paginated.data);
                if (filtered.isEmpty) {
                  return Center(
                    child: Padding(
                      padding: const EdgeInsets.all(32),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(Icons.inbox_outlined,
                              size: 56, color: theme.hintColor),
                          const SizedBox(height: 12),
                          Text(
                            _filter == null
                                ? 'Belum ada tiket.'
                                : 'Tidak ada tiket dengan status "${_filters.firstWhere((f) => f.$1 == _filter).$2}".',
                            textAlign: TextAlign.center,
                            style: theme.textTheme.bodyMedium?.copyWith(
                              color: theme.hintColor,
                            ),
                          ),
                        ],
                      ),
                    ),
                  );
                }
                return RefreshIndicator(
                  onRefresh: () async {
                    ref.invalidate(myTicketsProvider);
                    await Future.delayed(const Duration(milliseconds: 500));
                  },
                  child: ListView.builder(
                    itemCount: filtered.length,
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    itemBuilder: (_, i) {
                      final t = filtered[i];
                      return TicketCard(
                        ticket: t,
                        onTap: () => context.go('/tickets/${t.id}'),
                      );
                    },
                  ),
                );
              },
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (e, _) => Center(
                child: Padding(
                  padding: const EdgeInsets.all(24),
                  child: Text('Gagal memuat tiket: $e'),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  List<TicketModel> _applyFilter(List<TicketModel> tickets) {
    if (_filter == null) return tickets;
    return tickets.where((t) => t.status.name == _filter).toList();
  }
}