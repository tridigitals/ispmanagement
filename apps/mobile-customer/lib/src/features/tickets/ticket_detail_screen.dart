import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../../l10n/app_localizations.dart';
import '../../../services/service_providers.dart';

final ticketByIdProvider = FutureProvider.family<TicketModel, String>((ref, id) async {
  final svc = ref.watch(ticketServiceProvider);
  final res = await svc.getById(id);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw exception.message,
  };
});

final ticketMessagesProvider =
    FutureProvider.family<List<TicketMessageModel>, String>((ref, id) async {
  final svc = ref.watch(ticketServiceProvider);
  final res = await svc.listMessages(id);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw exception.message,
  };
});

class TicketDetailScreen extends ConsumerStatefulWidget {
  const TicketDetailScreen({required this.id, super.key});
  final String id;

  @override
  ConsumerState<TicketDetailScreen> createState() => _TicketDetailScreenState();
}

class _TicketDetailScreenState extends ConsumerState<TicketDetailScreen> {
  final _messageCtrl = TextEditingController();
  bool _sending = false;

  @override
  void dispose() {
    _messageCtrl.dispose();
    super.dispose();
  }

  Future<void> _send() async {
    final text = _messageCtrl.text.trim();
    if (text.isEmpty) return;
    setState(() => _sending = true);
    final res = await ref
        .read(ticketServiceProvider)
        .reply(ticketId: widget.id, body: text);
    if (!mounted) return;
    setState(() => _sending = false);
    switch (res) {
      case Success():
        _messageCtrl.clear();
        ref.invalidate(ticketMessagesProvider(widget.id));
      case Failure(:final exception):
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(exception.message)),
        );
    }
  }

  @override
  Widget build(BuildContext context) {
    final ticketAsync = ref.watch(ticketByIdProvider(widget.id));
    final messagesAsync = ref.watch(ticketMessagesProvider(widget.id));
    final dateFmt = DateFormat('d MMM yyyy HH:mm', 'id_ID');

    return Scaffold(
      appBar: AppBar(
        title: ticketAsync.maybeWhen(
          data: (t) => Text(t.subject, overflow: TextOverflow.ellipsis),
          orElse: () => const Text('Tiket'),
        ),
      ),
      body: Column(
        children: [
          Expanded(
            child: messagesAsync.when(
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (e, _) => Center(child: Text(e.toString())),
              data: (messages) {
                if (messages.isEmpty) {
                  return const Center(child: Text('Belum ada pesan'));
                }
                return ListView.builder(
                  padding: const EdgeInsets.all(16),
                  itemCount: messages.length,
                  itemBuilder: (_, i) {
                    final m = messages[i];
                    return Align(
                      alignment:
                          m.isFromStaff ? Alignment.centerLeft : Alignment.centerRight,
                      child: Container(
                        margin: const EdgeInsets.symmetric(vertical: 4),
                        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                        constraints: const BoxConstraints(maxWidth: 280),
                        decoration: BoxDecoration(
                          color: m.isFromStaff
                              ? IspColors.bgTertiary
                              : IspColors.primary,
                          borderRadius: BorderRadius.circular(IspRadii.lg),
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                              m.body,
                              style: TextStyle(
                                color: m.isFromStaff
                                    ? IspColors.textPrimary
                                    : Colors.white,
                              ),
                            ),
                            const SizedBox(height: 4),
                            Text(
                              '${m.authorName} · ${dateFmt.format(m.createdAt)}',
                              style: TextStyle(
                                fontSize: 10,
                                color: m.isFromStaff
                                    ? IspColors.textTertiary
                                    : Colors.white70,
                              ),
                            ),
                          ],
                        ),
                      ),
                    );
                  },
                );
              },
            ),
          ),
          SafeArea(
            top: false,
            child: Container(
              padding: const EdgeInsets.all(12),
              decoration: const BoxDecoration(
                color: IspColors.bgSecondary,
                border: Border(top: BorderSide(color: IspColors.borderSubtle)),
              ),
              child: Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _messageCtrl,
                      minLines: 1,
                      maxLines: 4,
                      decoration: const InputDecoration(
                        hintText: 'Tulis pesan...',
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  IconButton.filled(
                    onPressed: _sending ? null : _send,
                    icon: _sending
                        ? const SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.send),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
