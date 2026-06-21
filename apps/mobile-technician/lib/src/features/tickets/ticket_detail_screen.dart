import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';
import 'package:api_client/api_client.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../services/service_providers.dart';
import '../../services/ticket_providers.dart';
import '../../widgets/resolve_ticket_dialog.dart';
import '../../services/ticket_actions.dart';

/// Full ticket view with action bar.
/// Open → "Mulai Kerjakan"
/// In Progress → "Selesaikan" + "Tambah Catatan"
/// Resolved/Closed → read-only
class TicketDetailScreen extends ConsumerWidget {
  const TicketDetailScreen({super.key, required this.ticketId});
  final String ticketId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final ticketAsync = ref.watch(ticketByIdProvider(ticketId));
    final messagesAsync = ref.watch(ticketMessagesProvider(ticketId));

    return Scaffold(
      appBar: AppBar(
        title: const Text('Detail Tiket'),
        backgroundColor: theme.colorScheme.primary,
        foregroundColor: theme.colorScheme.onPrimary,
      ),
      body: ticketAsync.when(
        data: (ticket) => _Body(
          ticket: ticket,
          messagesAsync: messagesAsync,
          onChanged: () {
            ref.invalidate(ticketByIdProvider(ticketId));
            ref.invalidate(ticketMessagesProvider(ticketId));
            ref.invalidate(myTicketsProvider);
          },
        ),
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('Error: $e')),
      ),
    );
  }
}

class _Body extends ConsumerWidget {
  const _Body({
    required this.ticket,
    required this.messagesAsync,
    required this.onChanged,
  });
  final TicketModel ticket;
  final AsyncValue<List<TicketMessageModel>> messagesAsync;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final canAct = ticket.status == TicketStatus.open ||
        ticket.status == TicketStatus.inProgress;

    return Stack(
      children: [
        ListView(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 100),
          children: [
            // Header: subject + status pill
            Text(
              ticket.subject,
              style: theme.textTheme.headlineSmall?.copyWith(
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              runSpacing: 6,
              children: [
                _Pill(
                  label: ticket.priority.name.toUpperCase(),
                  color: _priorityColor(ticket.priority),
                ),
                _Pill(
                  label: ticket.statusLabel(),
                  color: _statusColor(ticket.status),
                ),
                if (ticket.category != null)
                  _Pill(label: ticket.category!, color: Colors.grey),
              ],
            ),
            const SizedBox(height: 16),

            // Description
            if (ticket.description != null && ticket.description!.isNotEmpty) ...[
              Text('Deskripsi', style: theme.textTheme.titleSmall),
              const SizedBox(height: 4),
              Card(
                elevation: 0,
                color: theme.colorScheme.surfaceContainerHighest,
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Text(ticket.description!),
                ),
              ),
              const SizedBox(height: 16),
            ],

            // Metadata
            _MetaRow(label: 'Tanggal Dibuat',
                value: DateFormat('d MMM yyyy, HH:mm').format(ticket.createdAt.toLocal())),
            if (ticket.assignedToName != null)
              _MetaRow(label: 'Ditugaskan ke', value: ticket.assignedToName!),
            const SizedBox(height: 16),

            // Messages thread
            Text('Percakapan', style: theme.textTheme.titleSmall),
            const SizedBox(height: 8),
            messagesAsync.when(
              data: (msgs) => msgs.isEmpty
                  ? Padding(
                      padding: const EdgeInsets.all(16),
                      child: Text(
                        'Belum ada pesan.',
                        style: theme.textTheme.bodyMedium
                            ?.copyWith(color: theme.hintColor),
                      ),
                    )
                  : Column(
                      children: msgs
                          .map((m) => _MessageBubble(message: m))
                          .toList(),
                    ),
              loading: () => const Padding(
                padding: EdgeInsets.all(16),
                child: Center(child: CircularProgressIndicator()),
              ),
              error: (e, _) => Text('Gagal memuat pesan: $e'),
            ),
          ],
        ),
        // Bottom action bar (sticky)
        if (canAct)
          Positioned(
            left: 0,
            right: 0,
            bottom: 0,
            child: _ActionBar(
              ticket: ticket,
              onChanged: onChanged,
            ),
          ),
      ],
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

  Color _statusColor(TicketStatus s) {
    switch (s) {
      case TicketStatus.open:
        return Colors.blue;
      case TicketStatus.inProgress:
        return Colors.orange;
      case TicketStatus.resolved:
      case TicketStatus.closed:
        return Colors.green;
      case TicketStatus.cancelled:
        return Colors.red;
      default:
        return Colors.purple;
    }
  }
}

class _ActionBar extends ConsumerStatefulWidget {
  const _ActionBar({required this.ticket, required this.onChanged});
  final TicketModel ticket;
  final VoidCallback onChanged;

  @override
  ConsumerState<_ActionBar> createState() => _ActionBarState();
}

class _ActionBarState extends ConsumerState<_ActionBar> {
  bool _busy = false;

  Future<void> _startWork() async {
    setState(() => _busy = true);
    try {
      final updated = await ref
          .read(ticketActionControllerProvider)
          .start(widget.ticket.id);
      if (!mounted) return;
      if (updated != null) {
        widget.onChanged();
        ref.invalidate(ticketByIdProvider(widget.ticket.id));
        ref.invalidate(myTicketsProvider);
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Tiket dimulai — mulai bekerja')),
        );
      } else {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Gagal memulai tiket. Coba lagi.')),
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _openResolveDialog() async {
    await showDialog<bool>(
      context: context,
      builder: (_) => ResolveTicketDialog(ticketId: widget.ticket.id),
    );
    ref.invalidate(ticketByIdProvider(widget.ticket.id));
    ref.invalidate(myTicketsProvider);
    widget.onChanged();
  }

  Future<void> _addNote() async {
    final controller = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Tambah Catatan'),
        content: TextField(
          controller: controller,
          maxLines: 4,
          autofocus: true,
          decoration: const InputDecoration(
            hintText: 'Tulis catatan atau update...',
            border: OutlineInputBorder(),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Batal'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Kirim'),
          ),
        ],
      ),
    );
    if (ok != true || controller.text.trim().isEmpty) return;

    setState(() => _busy = true);
    try {
      final svc = ref.read(ticketServiceProvider);
      final res = await svc.reply(
        ticketId: widget.ticket.id,
        message: controller.text.trim(),
      );
      res.fold(
        (_) {
          widget.onChanged();
          if (mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(content: Text('Catatan ditambahkan')),
            );
          }
        },
        (err) {
          if (mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(content: Text('Gagal: ${err.message}')),
            );
          }
        },
      );
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final isOpen = widget.ticket.status == TicketStatus.open;
    return Container(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
      decoration: BoxDecoration(
        color: Theme.of(context).cardColor,
        boxShadow: [
          BoxShadow(
            color: Colors.black.withOpacity(0.08),
            blurRadius: 8,
            offset: const Offset(0, -2),
          ),
        ],
      ),
      child: Row(
        children: [
          if (isOpen)
            Expanded(
              child: FilledButton.icon(
                icon: const Icon(Icons.play_arrow),
                label: const Text('Mulai Kerjakan'),
                onPressed: _busy ? null : _startWork,
              ),
            )
          else
            Expanded(
              child: FilledButton.icon(
                icon: const Icon(Icons.check),
                label: const Text('Selesaikan'),
                onPressed: _busy ? null : _openResolveDialog,
              ),
            ),
          const SizedBox(width: 8),
          IconButton.outlined(
            icon: const Icon(Icons.add_comment),
            tooltip: 'Tambah Catatan',
            onPressed: _busy ? null : _addNote,
          ),
        ],
      ),
    );
  }
}

class _Pill extends StatelessWidget {
  const _Pill({required this.label, required this.color});
  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(20),
        border: Border.all(color: color.withOpacity(0.4)),
      ),
      child: Text(
        label.toUpperCase(),
        style: TextStyle(
          color: color,
          fontSize: 11,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }
}

class _MetaRow extends StatelessWidget {
  const _MetaRow({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 130,
            child: Text(
              label,
              style: theme.textTheme.bodySmall?.copyWith(color: theme.hintColor),
            ),
          ),
          Expanded(
            child: Text(value, style: theme.textTheme.bodyMedium),
          ),
        ],
      ),
    );
  }
}

class _MessageBubble extends StatelessWidget {
  const _MessageBubble({required this.message});
  final TicketMessageModel message;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.symmetric(vertical: 4),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text(
                message.authorName ?? message.authorId ?? 'Anonim',
                style: theme.textTheme.labelLarge?.copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
              const Spacer(),
              Text(
                DateFormat('d MMM HH:mm').format(message.createdAt.toLocal()),
                style: theme.textTheme.bodySmall?.copyWith(color: theme.hintColor),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(message.body),
        ],
      ),
    );
  }
}