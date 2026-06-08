import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

class TicketListScreen extends StatefulWidget {
  const TicketListScreen({super.key});

  @override
  State<TicketListScreen> createState() => _TicketListScreenState();
}

class _TicketListScreenState extends State<TicketListScreen> with SingleTickerProviderStateMixin {
  late TabController _tabCtrl;

  @override
  void initState() {
    super.initState();
    _tabCtrl = TabController(length: 4, vsync: this);
  }

  @override
  void dispose() {
    _tabCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Tiket Support'),
        bottom: TabBar(
          controller: _tabCtrl,
          isScrollable: true,
          tabs: const [
            Tab(text: 'Semua'),
            Tab(text: 'Terbuka'),
            Tab(text: 'Dikerjakan'),
            Tab(text: 'Selesai'),
          ],
        ),
      ),
      body: TabBarView(
        controller: _tabCtrl,
        children: List.generate(4, (_) => _TicketList()),
      ),
    );
  }
}

class _TicketList extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      itemCount: 0,
      itemBuilder: (context, index) => ListTile(
        leading: CircleAvatar(
          backgroundColor: Colors.orange.withValues(alpha: 0.15),
          child: const Icon(Icons.support_agent, color: Colors.orange, size: 20),
        ),
        title: Text('Tiket #\$index'),
        subtitle: Text('Pelanggan • 2h lalu'),
        trailing: const Icon(Icons.chevron_right),
        onTap: () => context.go('/tickets/\$index'),
      ),
    );
  }
}
