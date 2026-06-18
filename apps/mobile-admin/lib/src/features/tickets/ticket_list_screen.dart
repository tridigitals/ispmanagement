import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:mobile_admin/src/l10n/app_localizations.dart';
import 'package:mobile_admin/src/services/service_providers.dart';

import 'ticket_l10n.dart';

/// Filter status for tab selection. Null = All.
final _ticketFilterProvider = StateProvider<String?>((_) => null);

/// Admin can see all tickets (not just own). Returns paginated first page.
final adminTicketsProvider = FutureProvider.autoDispose
    .family<List<TicketModel>, String?>((ref, status) async {
  final svc = ref.watch(ticketServiceProvider);
  final result = await svc.list(status: status, perPage: 50);
  return result.getOrThrow().data;
});

class TicketListScreen extends ConsumerWidget {
  const TicketListScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final filter = ref.watch(_ticketFilterProvider);
    final asyncTickets = ref.watch(adminTicketsProvider(filter));

    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.ticketAdminListTitle),
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(48),
          child: Container(
            color: Theme.of(context).appBarTheme.backgroundColor,
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 8),
              child: Row(
                children: [
                  _FilterTab(
                    label: l10n.ticketAdminTabAll,
                    value: null,
                    groupValue: filter,
                    onChanged: (v) =>
                        ref.read(_ticketFilterProvider.notifier).state = v,
                  ),
                  _FilterTab(
                    label: l10n.ticketAdminTabOpen,
                    value: 'open',
                    groupValue: filter,
                    onChanged: (v) =>
                        ref.read(_ticketFilterProvider.notifier).state = v,
                  ),
                  _FilterTab(
                    label: l10n.ticketAdminTabInProgress,
                    value: 'in_progress',
                    groupValue: filter,
                    onChanged: (v) =>
                        ref.read(_ticketFilterProvider.notifier).state = v,
                  ),
                  _FilterTab(
                    label: l10n.ticketAdminTabClosed,
                    value: 'closed',
                    groupValue: filter,
                    onChanged: (v) =>
                        ref.read(_ticketFilterProvider.notifier).state = v,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
      body: RefreshIndicator(
        onRefresh: () async => ref.invalidate(adminTicketsProvider(filter)),
        child: asyncTickets.when(
          loading: () => const Center(child: CircularProgressIndicator()),
          error: (e, _) => _ErrorState(
            message: e.toString(),
            onRetry: () => ref.invalidate(adminTicketsProvider(filter)),
          ),
          data: (tickets) {
            if (tickets.isEmpty) {
              return ListView(
                // ListView so RefreshIndicator works on empty.
                children: [
                  const SizedBox(height: 80),
                  Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          Icons.inbox_outlined,
                          size: 56,
                          color: Theme.of(context).colorScheme.outline,
                        ),
                        const SizedBox(height: 12),
                        Text(
                          l10n.ticketAdminEmpty,
                          style: Theme.of(context).textTheme.bodyLarge,
                        ),
                      ],
                    ),
                  ),
                ],
              );
            }
            return ListView.separated(
              padding: const EdgeInsets.symmetric(vertical: 8),
              itemCount: tickets.length,
              separatorBuilder: (_, __) => const Divider(height: 1),
              itemBuilder: (_, i) => _TicketRow(ticket: tickets[i]),
            );
          },
        ),
      ),
    );
  }
}

class _FilterTab extends StatelessWidget {
  const _FilterTab({
    required this.label,
    required this.value,
    required this.groupValue,
    required this.onChanged,
  });
  final String label;
  final String? value;
  final String? groupValue;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context) {
    final selected = value == groupValue;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
      child: ChoiceChip(
        label: Text(label),
        selected: selected,
        onSelected: (_) => onChanged(value),
      ),
    );
  }
}

class _TicketRow extends ConsumerWidget {
  const _TicketRow({required this.ticket});
  final TicketModel ticket;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final dateFmt = DateFormat('d MMM HH:mm', 'id_ID');

    final tone = _statusTone(ticket.status);

    return ListTile(
      onTap: () => context.push('/tickets/${ticket.id}'),
      leading: CircleAvatar(
        backgroundColor: tone.withOpacity(0.15),
        child: Icon(_statusIcon(ticket.status), color: tone, size: 20),
      ),
      title: Text(
        ticket.subject,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Row(
        children: [
          Expanded(
            child: Text(
              '#${ticket.id.substring(0, 8)}',
              style: theme.textTheme.bodySmall,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            '· ${dateFmt.format(ticket.updatedAt)}',
            style: theme.textTheme.bodySmall,
          ),
        ],
      ),
      trailing: Wrap(
        spacing: 4,
        children: [
          if (ticket.category != null && ticket.category!.isNotEmpty)
            _MiniBadge(
              label: l10n.ticketCategoryLabel(ticket.category),
              color: theme.colorScheme.outline,
            ),
          _MiniBadge(
            label: l10n.ticketStatusLabel(ticket.status),
            color: tone,
          ),
        ],
      ),
      isThreeLine: false,
    );
  }

  Color _statusTone(TicketStatus s) {
    switch (s) {
      case TicketStatus.open:
        return Colors.orange;
      case TicketStatus.inProgress:
        return Colors.blue;
      case TicketStatus.waitingCustomer:
        return Colors.purple;
      case TicketStatus.waitingStaff:
        return Colors.teal;
      case TicketStatus.resolved:
      case TicketStatus.closed:
        return Colors.green;
      case TicketStatus.cancelled:
        return Colors.grey;
    }
  }

  IconData _statusIcon(TicketStatus s) {
    switch (s) {
      case TicketStatus.open:
        return Icons.support_agent;
      case TicketStatus.inProgress:
        return Icons.build;
      case TicketStatus.waitingCustomer:
        return Icons.hourglass_top;
      case TicketStatus.waitingStaff:
        return Icons.hourglass_bottom;
      case TicketStatus.resolved:
        return Icons.task_alt;
      case TicketStatus.closed:
        return Icons.lock;
      case TicketStatus.cancelled:
        return Icons.block;
    }
  }
}

class _MiniBadge extends StatelessWidget {
  const _MiniBadge({required this.label, required this.color});
  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 10,
          fontWeight: FontWeight.w600,
          color: color,
        ),
      ),
    );
  }
}

class _ErrorState extends StatelessWidget {
  const _ErrorState({required this.message, required this.onRetry});
  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.error_outline,
              size: 48,
              color: Theme.of(context).colorScheme.error,
            ),
            const SizedBox(height: 12),
            Text(message, textAlign: TextAlign.center),
            const SizedBox(height: 12),
            FilledButton.icon(
              onPressed: onRetry,
              icon: const Icon(Icons.refresh),
              label: Text(AppLocalizations.of(context).retry),
            ),
          ],
        ),
      ),
    );
  }
}
