import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart' show ticketServiceProvider;
import '../../theme/app_theme.dart';
import '../tickets/ticket_l10n.dart';

class TicketsTab extends ConsumerStatefulWidget {
  const TicketsTab({super.key});

  @override
  ConsumerState<TicketsTab> createState() => _TicketsTabState();
}

class _TicketsTabState extends ConsumerState<TicketsTab> {
  final List<TicketModel> _items = [];
  int _page = 1;
  bool _hasMore = true;
  bool _loadingMore = false;
  bool _initialLoaded = false;
  Object? _initialError;
  TicketStatus? _statusFilter;

  static const _perPage = 20;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadInitial());
  }

  Future<void> _loadInitial() async {
    try {
      final svc = ref.read(ticketServiceProvider);
      final result = await svc.list(
        page: 1,
        perPage: _perPage,
        status: _statusFilter?.apiValue(),
      );
      final paginated = result.getOrThrow();
      if (!mounted) return;
      setState(() {
        _items
          ..clear()
          ..addAll(paginated.data);
        _hasMore = paginated.hasMore;
        _page = 1;
        _initialLoaded = true;
        _initialError = null;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _initialError = e;
        _initialLoaded = true;
      });
    }
  }

  Future<void> _loadMore() async {
    if (_loadingMore || !_hasMore) return;
    setState(() => _loadingMore = true);
    try {
      final svc = ref.read(ticketServiceProvider);
      final result = await svc.list(
        page: _page + 1,
        perPage: _perPage,
        status: _statusFilter?.apiValue(),
      );
      final paginated = result.getOrThrow();
      if (!mounted) return;
      setState(() {
        _items.addAll(paginated.data);
        _hasMore = paginated.hasMore;
        _page++;
        _loadingMore = false;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() => _loadingMore = false);
    }
  }

  void _onFilterChanged(TicketStatus? status) {
    if (status == _statusFilter) return;
    setState(() {
      _statusFilter = status;
      _initialLoaded = false;
      _items.clear();
      _page = 1;
      _hasMore = true;
    });
    _loadInitial();
  }

  Widget _buildFilterChips(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final filters = <(String, TicketStatus?)>[
      (l10n.ticketAdminTabAll, null),
      (l10n.ticketStatusOpen, TicketStatus.open),
      (l10n.ticketStatusInProgress, TicketStatus.inProgress),
      (l10n.ticketStatusWaitingCustomer, TicketStatus.waitingCustomer),
      (l10n.ticketStatusResolved, TicketStatus.resolved),
    ];

    return SizedBox(
      height: 44,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 16),
        itemCount: filters.length,
        separatorBuilder: (_, __) => const SizedBox(width: 8),
        itemBuilder: (_, i) {
          final (label, status) = filters[i];
          final selected = _statusFilter == status;
          return FilterChip(
            label: Text(label),
            selected: selected,
            onSelected: (_) => _onFilterChanged(status),
            selectedColor: isp.accent.withOpacity(0.15),
            checkmarkColor: isp.accent,
            labelStyle: TextStyle(
              fontSize: 13,
              color: selected ? isp.accent : isp.textSecondary,
              fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
            ),
            side: BorderSide(
              color: selected ? isp.accent : isp.border,
            ),
            backgroundColor: isp.surface,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(20),
            ),
          );
        },
      ),
    );
  }

  Widget _buildTicketTile(BuildContext context, TicketModel ticket) {
    final isp = context.isp;

    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: () => GoRouter.of(context).push('/tickets/${ticket.id}'),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          decoration: BoxDecoration(
            border: Border(
              bottom: BorderSide(color: isp.border, width: 0.5),
            ),
          ),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Priority color stripe
              Container(
                width: 4,
                height: 48,
                decoration: BoxDecoration(
                  color: _priorityColor(ticket.priority),
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const SizedBox(width: 12),
              // Content
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      ticket.subject,
                      style: TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w600,
                        color: isp.textPrimary,
                      ),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 4),
                    Row(
                      children: [
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 6,
                            vertical: 2,
                          ),
                          decoration: BoxDecoration(
                            color: ticket.statusColor().withOpacity(0.12),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text(
                            ticket.statusLabel(),
                            style: TextStyle(
                              fontSize: 11,
                              fontWeight: FontWeight.w600,
                              color: ticket.statusColor(),
                            ),
                          ),
                        ),
                        const SizedBox(width: 8),
                        Text(
                          '#${ticket.id.substring(0, 8)}',
                          style: TextStyle(
                            fontSize: 12,
                            color: isp.textMuted,
                          ),
                        ),
                        if (ticket.assignedToName != null) ...[
                          const SizedBox(width: 8),
                          Icon(Icons.person_outline,
                              size: 12, color: isp.textMuted),
                          const SizedBox(width: 3),
                          Expanded(
                            child: Text(
                              ticket.assignedToName!,
                              style: TextStyle(
                                fontSize: 12,
                                color: isp.textMuted,
                              ),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ],
                      ],
                    ),
                  ],
                ),
              ),
              // Priority label
              const SizedBox(width: 8),
              _MiniPriorityLabel(priority: ticket.priority),
            ],
          ),
        ),
      ),
    );
  }

  Color _priorityColor(TicketPriority p) {
    switch (p) {
      case TicketPriority.urgent:
        return Colors.red;
      case TicketPriority.high:
        return Colors.orange;
      case TicketPriority.normal:
        return Colors.blue;
      case TicketPriority.low:
        return Colors.grey;
    }
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Column(
      children: [
        const SizedBox(height: 8),
        // Sticky filter chips
        _buildFilterChips(context),
        const SizedBox(height: 8),
        // Body — loading / error / empty / list
        Expanded(
          child: _buildBody(context),
        ),
      ],
    );
  }

  Widget _buildBody(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    if (!_initialLoaded) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_initialError != null && _items.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.error_outline, size: 48, color: isp.textMuted),
              const SizedBox(height: 12),
              Text(
                l10n.ticketErrorLoadFailed,
                style: TextStyle(color: isp.textMuted),
              ),
              const SizedBox(height: 12),
              ElevatedButton(
                onPressed: () {
                  setState(() => _initialLoaded = false);
                  _loadInitial();
                },
                child: Text(l10n.retry),
              ),
            ],
          ),
        ),
      );
    }

    return RefreshIndicator(
      onRefresh: () async {
        _onFilterChanged(_statusFilter);
      },
      color: isp.accent,
      child: _items.isEmpty
          ? ListView(
              children: [
                SizedBox(
                    height:
                        MediaQuery.of(context).size.height * 0.25),
                Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.check_circle_outline,
                          size: 48, color: isp.textMuted),
                      const SizedBox(height: 8),
                      Text(
                        l10n.noAssignedTickets,
                        style: TextStyle(
                          color: isp.textMuted,
                          fontSize: 16,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            )
          : ListView.builder(
              itemCount: _items.length + (_hasMore ? 1 : 0),
              itemBuilder: (_, i) {
                if (i == _items.length) {
                  if (_loadingMore) {
                    return const Padding(
                      padding: EdgeInsets.all(16),
                      child:
                          Center(child: CircularProgressIndicator()),
                    );
                  }
                  _loadMore();
                  return const SizedBox.shrink();
                }
                return _buildTicketTile(context, _items[i]);
              },
            ),
    );
  }
}

// ─── Helpers ────────────────────────────────────────────────────

extension _TicketStatusApi on TicketStatus {
  String apiValue() {
    switch (this) {
      case TicketStatus.open:
        return 'open';
      case TicketStatus.inProgress:
        return 'in_progress';
      case TicketStatus.waitingCustomer:
        return 'waiting_customer';
      case TicketStatus.waitingStaff:
        return 'waiting_staff';
      case TicketStatus.resolved:
        return 'resolved';
      case TicketStatus.closed:
        return 'closed';
      case TicketStatus.cancelled:
        return 'cancelled';
    }
  }
}

extension _StatusDisplay on TicketModel {
  String statusLabel() {
    switch (status) {
      case TicketStatus.open:
        return 'Open';
      case TicketStatus.inProgress:
        return 'In Progress';
      case TicketStatus.waitingCustomer:
        return 'Waiting';
      case TicketStatus.waitingStaff:
        return 'Waiting';
      case TicketStatus.resolved:
        return 'Resolved';
      case TicketStatus.closed:
        return 'Closed';
      case TicketStatus.cancelled:
        return 'Cancelled';
    }
  }

  Color statusColor() {
    switch (status) {
      case TicketStatus.open:
        return Colors.orange;
      case TicketStatus.inProgress:
        return Colors.blue;
      case TicketStatus.waitingCustomer:
        return Colors.purple;
      case TicketStatus.waitingStaff:
        return Colors.teal;
      case TicketStatus.resolved:
        return Colors.green;
      case TicketStatus.closed:
      case TicketStatus.cancelled:
        return Colors.grey;
    }
  }
}

class _MiniPriorityLabel extends StatelessWidget {
  const _MiniPriorityLabel({required this.priority});
  final TicketPriority priority;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final (label, color) = _priorityInfo(priority);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withOpacity(0.1),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 10,
          fontWeight: FontWeight.w700,
          color: color,
        ),
      ),
    );
  }
}

(String, Color) _priorityInfo(TicketPriority priority) {
  switch (priority) {
    case TicketPriority.urgent:
      return ('URG', Colors.red);
    case TicketPriority.high:
      return ('HI', Colors.orange);
    case TicketPriority.normal:
      return ('MED', Colors.blue);
    case TicketPriority.low:
      return ('LOW', Colors.grey);
  }
}
