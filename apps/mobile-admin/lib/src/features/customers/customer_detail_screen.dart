import 'package:flutter/material.dart';

class CustomerDetailScreen extends StatelessWidget {
  final String customerId;
  const CustomerDetailScreen({super.key, required this.customerId});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('Pelanggan #\$customerId'),
        actions: [
          IconButton(icon: const Icon(Icons.edit), onPressed: () {}),
          IconButton(icon: const Icon(Icons.more_vert), onPressed: () {}),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                children: [
                  const CircleAvatar(radius: 32, child: Icon(Icons.person, size: 32)),
                  const SizedBox(height: 12),
                  Text('Nama Pelanggan', style: Theme.of(context).textTheme.titleLarge),
                  Text('email@example.com', style: Theme.of(context).textTheme.bodyMedium),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),
          Card(
            child: Column(
              children: [
                ListTile(leading: const Icon(Icons.phone), title: const Text('Telepon'), subtitle: const Text('+62 xxx')),
                const Divider(height: 1),
                ListTile(leading: const Icon(Icons.location_on), title: const Text('Alamat'), subtitle: const Text('—')),
                const Divider(height: 1),
                ListTile(leading: const Icon(Icons.router), title: const Text('Perangkat'), subtitle: const Text('—')),
                const Divider(height: 1),
                ListTile(leading: const Icon(Icons.wifi), title: const Text('Paket'), subtitle: const Text('—')),
              ],
            ),
          ),
          const SizedBox(height: 16),
          Text('Riwayat', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Card(
            child: Column(
              children: [
                ListTile(leading: const Icon(Icons.receipt), title: const Text('Invoice'), trailing: const Icon(Icons.chevron_right), onTap: () {}),
                const Divider(height: 1),
                ListTile(leading: const Icon(Icons.payment), title: const Text('Pembayaran'), trailing: const Icon(Icons.chevron_right), onTap: () {}),
                const Divider(height: 1),
                ListTile(leading: const Icon(Icons.support_agent), title: const Text('Tiket'), trailing: const Icon(Icons.chevron_right), onTap: () {}),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
