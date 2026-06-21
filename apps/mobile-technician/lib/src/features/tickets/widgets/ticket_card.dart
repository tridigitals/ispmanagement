import 'package:flutter/material.dart';
import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

/// Ticket card widget — displays a single ticket in a list.
class TicketCard extends StatelessWidget {
  const TicketCard({super.key, required this.ticket, this.onTap});

  final TicketModel ticket;
  final VoidCallback? onTap;

  Color _statusColor(String status) {
    switch (status) {
      case 'open':
        return const Color(0xFFF59E0B);
      case 'inProgress':
        return const Color(0xFF3B82F6);
      case 'resolved':
      case 'closed':
        return const Color(0xFF22C55E);
      default:
        return const Color(0xFF6B7280);
    }
  }

  String _statusLabel(String status) {
    switch (status) {
      case 'open':
        return 'Open';
      case 'inProgress':
        return 'Diproses';
      case 'resolved':
        return 'Selesai';
      case 'closed':
        return 'Ditutup';
      default:
        return status;
    }
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final statusColor = _statusColor(ticket.status.name);

    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(
                      ticket.subject,
                      style: TextStyle(
                        fontSize: 14,
                        fontWeight: FontWeight.w600,
                        color: isp.textPrimary,
                      ),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Container(
                    padding:
                        const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                    decoration: BoxDecoration(
                      color: statusColor.withOpacity(0.12),
                      borderRadius: BorderRadius.circular(IspRadii.pill),
                    ),
                    child: Text(
                      _statusLabel(ticket.status.name),
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        color: statusColor,
                      ),
                    ),
                  ),
                ],
              ),
              if (ticket.description != null && ticket.description!.isNotEmpty) ...[
                const SizedBox(height: 6),
                Text(
                  ticket.description ?? '',
                  style: TextStyle(fontSize: 12, color: isp.textMuted),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
              const SizedBox(height: 8),
              Row(
                children: [
                  Icon(Icons.access_time, size: 14, color: isp.textMuted),
                  const SizedBox(width: 4),
                  Text(
                    ticket.createdAt != null
                        ? _formatDate(ticket.createdAt!)
                        : '',
                    style: TextStyle(fontSize: 11, color: isp.textMuted),
                  ),
                  if (ticket.priority.name != 'normal') ...[
                    const SizedBox(width: 12),
                    Icon(Icons.flag, size: 14, color: _priorityColor(ticket.priority.name)),
                    const SizedBox(width: 4),
                    Text(
                      ticket.priority.name.toUpperCase(),
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        color: _priorityColor(ticket.priority.name),
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

  Color _priorityColor(String priority) {
    switch (priority) {
      case 'urgent':
        return const Color(0xFFEF4444);
      case 'high':
        return const Color(0xFFF97316);
      case 'low':
        return const Color(0xFF6B7280);
      default:
        return const Color(0xFF6B7280);
    }
  }

  String _formatDate(DateTime date) {
    final now = DateTime.now();
    final diff = now.difference(date);
    if (diff.inMinutes < 1) return 'Baru saja';
    if (diff.inMinutes < 60) return '${diff.inMinutes}m lalu';
    if (diff.inHours < 24) return '${diff.inHours}j lalu';
    if (diff.inDays < 7) return '${diff.inDays}h lalu';
    return '${date.day}/${date.month}/${date.year}';
  }
}
