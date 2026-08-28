import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:mobile_admin/src/l10n/app_localizations.dart';
import 'package:mobile_admin/src/services/auth_providers.dart';
import 'package:mobile_admin/src/services/service_providers.dart';

import 'ticket_l10n.dart';

/// Single ticket by id.
final _ticketByIdProvider =
    FutureProvider.family<TicketModel, String>((ref, id) async {
  final svc = ref.watch(ticketServiceProvider);
  final result = await svc.getById(id);
  return result.getOrThrow();
});

/// Messages for a ticket.
final _ticketMessagesProvider =
    FutureProvider.family<List<TicketMessageModel>, String>((ref, id) async {
  final svc = ref.watch(ticketServiceProvider);
  final userId = ref.watch(_currentUserIdProvider);
  final result = await svc.listMessages(id, currentUserId: userId);
  return result.getOrThrow();
});

final _currentUserIdProvider = Provider<String?>((ref) {
  final auth = ref.watch(authProvider);
  return auth.user?['id'] as String?;
});

class TicketDetailScreen extends ConsumerStatefulWidget {
  const TicketDetailScreen({super.key, required this.ticketId});
  final String ticketId;

  @override
  ConsumerState<TicketDetailScreen> createState() => _TicketDetailScreenState();
}

class _TicketDetailScreenState extends ConsumerState<TicketDetailScreen> {
  final _replyCtrl = TextEditingController();
  final _scrollCtrl = ScrollController();
  bool _sending = false;

  @override
  void dispose() {
    _replyCtrl.dispose();
    _scrollCtrl.dispose();
    super.dispose();
  }

  Future<void> _sendReply() async {
    final text = _replyCtrl.text.trim();
    if (text.isEmpty) return;
    setState(() => _sending = true);
    final l10n = AppLocalizations.of(context);
    final messenger = ScaffoldMessenger.of(context);
    try {
      final svc = ref.read(ticketServiceProvider);
      final res = await svc.reply(
        ticketId: widget.ticketId,
        message: text,
      );
      if (!mounted) return;
      switch (res) {
        case Success():
          _replyCtrl.clear();
          ref.invalidate(_ticketMessagesProvider(widget.ticketId));
          messenger.showSnackBar(SnackBar(content: Text(l10n.ticketToastReplySent)));
          // Scroll to bottom on next frame.
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (_scrollCtrl.hasClients) {
              _scrollCtrl.animateTo(
                _scrollCtrl.position.maxScrollExtent,
                duration: const Duration(milliseconds: 200),
                curve: Curves.easeOut,
              );
            }
          });
        case Failure(:final exception):
          messenger.showSnackBar(
            SnackBar(
              content: Text(exception.message),
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
          );
      }
    } catch (e) {
      messenger.showSnackBar(
        SnackBar(content: Text(l10n.ticketErrorReplyFailed(e.toString()))),
      );
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  Future<void> _updateStatus(TicketStatus newStatus) async {
    final l10n = AppLocalizations.of(context);
    final messenger = ScaffoldMessenger.of(context);
    try {
      final svc = ref.read(ticketServiceProvider);
      final res = await svc.update(widget.ticketId, status: newStatus.name);
      if (!mounted) return;
      switch (res) {
        case Success():
          ref.invalidate(_ticketByIdProvider(widget.ticketId));
          ref.invalidate(_ticketMessagesProvider(widget.ticketId));
          messenger.showSnackBar(
            SnackBar(content: Text(l10n.ticketToastClosed)),
          );
        case Failure(:final exception):
          messenger.showSnackBar(SnackBar(content: Text(exception.message)));
      }
    } catch (e) {
      messenger.showSnackBar(
        SnackBar(content: Text(l10n.ticketErrorLoadFailed)),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final ticketAsync = ref.watch(_ticketByIdProvider(widget.ticketId));
    final messagesAsync = ref.watch(_ticketMessagesProvider(widget.ticketId));
    final currentUserId = ref.watch(_currentUserIdProvider);
    final dateFmt = DateFormat('d MMM yyyy HH:mm', 'id_ID');

    return Scaffold(
      appBar: AppBar(
        title: ticketAsync.maybeWhen(
          data: (t) => Text(
            '#${t.id.substring(0, 8)}',
            style: const TextStyle(fontFamily: 'monospace'),
          ),
          orElse: () => Text(l10n.ticketAdminListTitle),
        ),
        actions: [
          ticketAsync.maybeWhen(
            data: (t) => PopupMenuButton<String>(
              icon: const Icon(Icons.more_vert),
              onSelected: (action) async {
                switch (action) {
                  case 'close':
                    await _updateStatus(TicketStatus.closed);
                  case 'reopen':
                    await _updateStatus(TicketStatus.open);
                }
              },
              itemBuilder: (_) => [
                if (t.isOpen)
                  PopupMenuItem(
                    value: 'close',
                    child: Row(
                      children: [
                        const Icon(Icons.lock_outline, size: 18),
                        const SizedBox(width: 8),
                        Text(l10n.ticketButtonClose),
                      ],
                    ),
                  ),
                if (t.isClosed)
                  PopupMenuItem(
                    value: 'reopen',
                    child: Row(
                      children: [
                        const Icon(Icons.lock_open, size: 18),
                        const SizedBox(width: 8),
                        Text(l10n.ticketButtonReopen),
                      ],
                    ),
                  ),
              ],
            ),
            orElse: () => const SizedBox.shrink(),
          ),
          IconButton(
            icon: const Icon(Icons.refresh),
            tooltip: l10n.retry,
            onPressed: () {
              ref.invalidate(_ticketByIdProvider(widget.ticketId));
              ref.invalidate(_ticketMessagesProvider(widget.ticketId));
            },
          ),
        ],
      ),
      body: ticketAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.error_outline,
                    size: 48, color: theme.colorScheme.error),
                const SizedBox(height: 12),
                Text(e.toString(), textAlign: TextAlign.center),
                const SizedBox(height: 12),
                FilledButton.icon(
                  onPressed: () =>
                      ref.invalidate(_ticketByIdProvider(widget.ticketId)),
                  icon: const Icon(Icons.refresh),
                  label: Text(l10n.retry),
                ),
              ],
            ),
          ),
        ),
        data: (ticket) {
          return Column(
            children: [
              // ── Header card with status / priority / category ──
              _HeaderCard(ticket: ticket, dateFmt: dateFmt),
              // ── Conversation thread ──
              Expanded(
                child: messagesAsync.when(
                  loading: () => const Center(
                    child: CircularProgressIndicator(),
                  ),
                  error: (e, _) => Center(child: Text(e.toString())),
                  data: (messages) {
                    if (messages.isEmpty) {
                      return Center(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(Icons.chat_bubble_outline,
                                size: 48, color: theme.colorScheme.outline),
                            const SizedBox(height: 8),
                            Text(l10n.ticketNoMessages),
                          ],
                        ),
                      );
                    }
                    return ListView.builder(
                      controller: _scrollCtrl,
                      padding: const EdgeInsets.all(12),
                      itemCount: messages.length,
                      itemBuilder: (_, i) {
                        final m = messages[i];
                        final mine = m.authorId != null &&
                            m.authorId == currentUserId;
                        return _MessageBubble(
                          message: m,
                          isMine: mine,
                          dateFmt: dateFmt,
                        );
                      },
                    );
                  },
                ),
              ),
              // ── Reply bar ──
              _ReplyBar(
                controller: _replyCtrl,
                sending: _sending,
                enabled: ticket.isOpen,
                onSend: _sendReply,
                onAttach: () {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: Text(
                        '${l10n.ticketButtonAttach} — coming soon',
                      ),
                    ),
                  );
                },
              ),
            ],
          );
        },
      ),
    );
  }
}

class _HeaderCard extends ConsumerWidget {
  const _HeaderCard({required this.ticket, required this.dateFmt});
  final TicketModel ticket;
  final DateFormat dateFmt;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        border: Border(
          bottom: BorderSide(color: theme.dividerColor),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            ticket.subject,
            style: theme.textTheme.titleLarge,
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 6,
            runSpacing: 4,
            children: [
              _StatusChip(
                label: l10n.ticketStatusLabel(ticket.status),
                color: _statusColor(ticket.status),
              ),
              _StatusChip(
                label: l10n.ticketPriorityLabel(ticket.priority),
                color: _priorityColor(ticket.priority),
              ),
              if (ticket.category != null && ticket.category!.isNotEmpty)
                _StatusChip(
                  label: l10n.ticketCategoryLabel(ticket.category),
                  color: theme.colorScheme.outline,
                ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            '${l10n.ticketAdminListTitle} · ${dateFmt.format(ticket.createdAt)}',
            style: theme.textTheme.bodySmall,
          ),
          if (ticket.assignedToName != null &&
              ticket.assignedToName!.isNotEmpty) ...[
            const SizedBox(height: 4),
            Row(
              children: [
                Icon(Icons.person_outline,
                    size: 14, color: theme.colorScheme.outline),
                const SizedBox(width: 4),
                Text(
                  '${l10n.ticketAssignee}: ${ticket.assignedToName}',
                  style: theme.textTheme.bodySmall,
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }

  Color _statusColor(TicketStatus s) {
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

  Color _priorityColor(TicketPriority p) {
    switch (p) {
      case TicketPriority.low:
        return Colors.grey;
      case TicketPriority.normal:
        return Colors.blue;
      case TicketPriority.high:
        return Colors.orange;
      case TicketPriority.urgent:
        return Colors.red;
    }
  }
}

class _StatusChip extends StatelessWidget {
  const _StatusChip({required this.label, required this.color});
  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: color,
        ),
      ),
    );
  }
}

class _MessageBubble extends ConsumerWidget {
  const _MessageBubble({
    required this.message,
    required this.isMine,
    required this.dateFmt,
  });
  final TicketMessageModel message;
  final bool isMine;
  final DateFormat dateFmt;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return Align(
      alignment: isMine ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.symmetric(vertical: 4),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        constraints: BoxConstraints(
          maxWidth: MediaQuery.of(context).size.width * 0.8,
        ),
        decoration: BoxDecoration(
          color: isMine
              ? theme.colorScheme.primary.withOpacity(0.15)
              : theme.colorScheme.surfaceContainerHighest,
          borderRadius: BorderRadius.only(
            topLeft: const Radius.circular(14),
            topRight: const Radius.circular(14),
            bottomLeft: Radius.circular(isMine ? 14 : 2),
            bottomRight: Radius.circular(isMine ? 2 : 14),
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              l10n.ticketAuthorLabel(
                message.authorName,
                isCurrentUser: isMine,
              ),
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w600,
                color: isMine
                    ? theme.colorScheme.primary
                    : theme.colorScheme.outline,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              message.body,
              style: theme.textTheme.bodyMedium,
            ),
            if (message.attachments.isNotEmpty) ...[
              const SizedBox(height: 6),
              Wrap(
                spacing: 4,
                runSpacing: 4,
                children: message.attachments
                    .map((a) => Chip(
                          label: Text(
                            a.originalName,
                            overflow: TextOverflow.ellipsis,
                          ),
                          avatar: Icon(
                            a.isImage ? Icons.image : Icons.attach_file,
                            size: 16,
                          ),
                          visualDensity: VisualDensity.compact,
                        ))
                    .toList(),
              ),
            ],
            const SizedBox(height: 4),
            Text(
              dateFmt.format(message.createdAt),
              style: theme.textTheme.bodySmall,
            ),
          ],
        ),
      ),
    );
  }
}

class _ReplyBar extends StatelessWidget {
  const _ReplyBar({
    required this.controller,
    required this.sending,
    required this.enabled,
    required this.onSend,
    required this.onAttach,
  });
  final TextEditingController controller;
  final bool sending;
  final bool enabled;
  final VoidCallback onSend;
  final VoidCallback onAttach;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);

    if (!enabled) {
      return Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHighest,
          border: Border(top: BorderSide(color: theme.dividerColor)),
        ),
        child: SafeArea(
          child: Row(
            children: [
              Icon(Icons.lock, size: 16, color: theme.colorScheme.outline),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  l10n.ticketClosedNotice,
                  style: theme.textTheme.bodySmall,
                ),
              ),
              TextButton.icon(
                onPressed: () {
                  // parent handles this — exposed via the actions menu
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text(l10n.ticketButtonReopen)),
                  );
                },
                icon: const Icon(Icons.lock_open, size: 18),
                label: Text(l10n.ticketButtonReopen),
              ),
            ],
          ),
        ),
      );
    }

    return Container(
      padding: EdgeInsets.fromLTRB(
        8,
        8,
        8,
        8 + MediaQuery.viewInsetsOf(context).bottom,
      ),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        border: Border(top: BorderSide(color: theme.dividerColor)),
      ),
      child: SafeArea(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            IconButton(
              onPressed: sending ? null : onAttach,
              icon: const Icon(Icons.attach_file),
              tooltip: l10n.ticketButtonAttach,
            ),
            Expanded(
              child: TextField(
                controller: controller,
                minLines: 1,
                maxLines: 4,
                enabled: !sending,
                textCapitalization: TextCapitalization.sentences,
                decoration: InputDecoration(
                  hintText: l10n.ticketFieldReply,
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(20),
                  ),
                  contentPadding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 10,
                  ),
                ),
                onSubmitted: (_) => sending ? null : onSend(),
              ),
            ),
            const SizedBox(width: 8),
            IconButton.filled(
              onPressed: sending ? null : onSend,
              icon: sending
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        color: Colors.white,
                      ),
                    )
                  : const Icon(Icons.send),
              tooltip: l10n.ticketButtonSubmitReply,
            ),
          ],
        ),
      ),
    );
  }
}
