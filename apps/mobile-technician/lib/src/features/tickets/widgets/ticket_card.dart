import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:api_client/api_client.dart';

/// Compact card used in ticket list and home screen.
/// Shows subject, customer name, address (if any), priority + status badges,
/// and time-ago of last update.
class TicketCard extends StatelessWidget {
  const TicketCard({
    super.key,
    required this.ticket,
    required this.onTap,
  });

  final TicketModel ticket;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final priorityColor = _priorityColor(theme, ticket.priority);
    final statusColor = _statusColor(theme, ticket.status);

    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: theme.dividerColor.withValues(alpha: 0.5)),
      ),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Top row: priority badge + status badge + time-ago
              Row(
                children: [
                  _PriorityBadge(priority: ticket.priority, color: priorityColor),
                  const SizedBox(width: 6),
                  _StatusBadge(status: ticket.status, color: statusColor),
                  const Spacer(),
                  Text(
                    _timeAgo(ticket.updatedAt),
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.hintColor,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 10),
              // Subject
              Text(
                ticket.subject,
                style: theme.textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
              // Description preview (optional)
              if (ticket.description != null && ticket.description!.isNotEmpty) ...[
                const SizedBox(height: 4),
                Text(
                  ticket.description!,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.hintColor,
                  ),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
              // Bottom row: assignee + unread count
              const SizedBox(height: 8),
              Row(
                children: [
                  Icon(Icons.person_outline,
                      size: 14, color: theme.hintColor),
                  const SizedBox(width: 4),
                  Expanded(
                    child: Text(
                      ticket.assignedToName ?? 'Belum ditugaskan',
                      style: theme.textTheme.bodySmall,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  if (ticket.unreadCount > 0) ...[
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
                      decoration: BoxDecoration(
                        color: theme.colorScheme.primary,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Text(
                        '${ticket.unreadCount}',
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 10,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Color _priorityColor(ThemeData theme, TicketPriority p) {
    switch (p) {
      case TicketPriority.urgent:
        return Colors.red.shade700;
      case TicketPriority.high:
        return Colors.orange.shade700;
      case TicketPriority.normal:
        return theme.colorScheme.primary;
      case TicketPriority.low:
        return Colors.grey.shade600;
    }
  }

  Color _statusColor(ThemeData theme, TicketStatus s) {
    switch (s) {
      case TicketStatus.open:
        return Colors.blue.shade700;
      case TicketStatus.inProgress:
        return Colors.orange.shade700;
      case TicketStatus.waitingCustomer:
      case TicketStatus.waitingStaff:
        return Colors.purple.shade700;
      case TicketStatus.resolved:
        return Colors.green.shade700;
      case TicketStatus.closed:
        return Colors.grey.shade600;
      case TicketStatus.cancelled:
        return Colors.red.shade400;
    }
  }

  String _timeAgo(DateTime t) {
    final diff = DateTime.now().difference(t);
    if (diff.inMinutes < 1) return 'baru saja';
    if (diff.inMinutes < 60) return '${diff.inMinutes}m';
    if (diff.inHours < 24) return '${diff.inHours}j';
    if (diff.inDays < 7) return '${diff.inDays}h';
    return DateFormat('d MMM').format(t);
  }
}

class _PriorityBadge extends StatelessWidget {
  const _PriorityBadge({required this.priority, required this.color});
  final TicketPriority priority;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: color.withValues(alpha: 0.4), width: 0.5),
      ),
      child: Text(
        priority.name.toUpperCase(),
        style: TextStyle(
          color: color,
          fontSize: 10,
          fontWeight: FontWeight.w700,
          letterSpacing: 0.5,
        ),
      ),
    );
  }
}

class _StatusBadge extends StatelessWidget {
  const _StatusBadge({required this.status, required this.color});
  final TicketStatus status;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        _statusLabel(status),
        style: TextStyle(
          color: color,
          fontSize: 10,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }

  String _statusLabel(TicketStatus s) {
    switch (s) {
      case TicketStatus.open:
        return 'OPEN';
      case TicketStatus.inProgress:
        return 'DIPROSES';
      case TicketStatus.waitingCustomer:
        return 'TUNGGU PELANGGAN';
      case TicketStatus.waitingStaff:
        return 'TUNGGU STAFF';
      case TicketStatus.resolved:
        return 'SELESAI';
      case TicketStatus.closed:
        return 'DITUTUP';
      case TicketStatus.cancelled:
        return 'DIBATALKAN';
    }
  }
}