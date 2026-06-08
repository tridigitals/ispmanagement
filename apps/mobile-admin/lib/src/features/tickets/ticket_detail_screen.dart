import 'package:flutter/material.dart';

class TicketDetailScreen extends StatelessWidget {
  final String ticketId;
  const TicketDetailScreen({super.key, required this.ticketId});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('Tiket #\$ticketId'),
        actions: [
          PopupMenuButton(
            itemBuilder: (context) => [
              const PopupMenuItem(value: 'assign', child: Text('Tugaskan')),
              const PopupMenuItem(value: 'close', child: Text('Tutup Tiket')),
              const PopupMenuItem(value: 'escalate', child: Text('Eskalasi')),
            ],
          ),
        ],
      ),
      body: Column(
        children: [
          Expanded(
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: [
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Chip(label: Text('Terbuka'), backgroundColor: Colors.orange.withValues(alpha: 0.15)),
                            const SizedBox(width: 8),
                            Chip(label: Text('Teknis')),
                          ],
                        ),
                        const SizedBox(height: 12),
                        Text('Judul Tiket', style: Theme.of(context).textTheme.titleLarge),
                        const SizedBox(height: 8),
                        Text('Deskripsi masalah pelanggan...', style: Theme.of(context).textTheme.bodyMedium),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 16),
                Text('Riwayat Chat', style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                _ChatBubble(isAdmin: false, message: 'Internet saya mati sejak kemarin', time: '10:30'),
                _ChatBubble(isAdmin: true, message: 'Baik, saya cek dulu ya', time: '10:45'),
              ],
            ),
          ),
          _ReplyBar(),
        ],
      ),
    );
  }
}

class _ChatBubble extends StatelessWidget {
  final bool isAdmin;
  final String message;
  final String time;

  const _ChatBubble({required this.isAdmin, required this.message, required this.time});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Align(
      alignment: isAdmin ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.only(bottom: 8),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        constraints: BoxConstraints(maxWidth: MediaQuery.of(context).size.width * 0.75),
        decoration: BoxDecoration(
          color: isAdmin
              ? theme.colorScheme.primary.withValues(alpha: 0.15)
              : theme.colorScheme.surfaceContainerHighest,
          borderRadius: BorderRadius.circular(16),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            Text(message, style: theme.textTheme.bodyMedium),
            const SizedBox(height: 4),
            Text(time, style: theme.textTheme.bodySmall),
          ],
        ),
      ),
    );
  }
}

class _ReplyBar extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        border: Border(top: BorderSide(color: Theme.of(context).dividerColor)),
      ),
      child: SafeArea(
        child: Row(
          children: [
            IconButton(icon: const Icon(Icons.attach_file), onPressed: () {}),
            Expanded(
              child: TextField(
                decoration: const InputDecoration(
                  hintText: 'Balas...',
                  border: OutlineInputBorder(borderRadius: BorderRadius.all(Radius.circular(24))),
                  contentPadding: EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                ),
              ),
            ),
            const SizedBox(width: 8),
            IconButton.filled(icon: const Icon(Icons.send), onPressed: () {}),
          ],
        ),
      ),
    );
  }
}
