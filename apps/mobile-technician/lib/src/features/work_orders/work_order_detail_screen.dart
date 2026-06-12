import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:api_client/api_client.dart';
import 'package:mobile_technician/src/services/auth_providers.dart';
import 'package:url_launcher/url_launcher.dart';

// ── Provider ──

final workOrderDetailProvider =
    FutureProvider.family<WorkOrderModel, String>((ref, id) async {
  final service = ref.watch(workOrderServiceProvider);
  final result = await service.getById(id);
  switch (result) {
    case Success(:final data):
      return data;
    case Failure(:final exception):
      throw Exception(exception.message);
  }
});

// ── Detail Screen ──

class WorkOrderDetailScreen extends ConsumerWidget {
  final String workOrderId;
  const WorkOrderDetailScreen({super.key, required this.workOrderId});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final woAsync = ref.watch(workOrderDetailProvider(workOrderId));
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: Text('WO #${workOrderId.substring(0, 8)}'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () =>
                ref.invalidate(workOrderDetailProvider(workOrderId)),
          ),
        ],
      ),
      body: woAsync.when(
        data: (wo) => _WorkOrderContent(workOrder: wo),
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.error_outline, size: 48, color: Colors.red),
              const SizedBox(height: 16),
              Text('Error: $e'),
              const SizedBox(height: 16),
              FilledButton(
                onPressed: () =>
                    ref.invalidate(workOrderDetailProvider(workOrderId)),
                child: const Text('Coba Lagi'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _WorkOrderContent extends ConsumerWidget {
  final WorkOrderModel workOrder;
  const _WorkOrderContent({required this.workOrder});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // ── Status Card ──
        _StatusCard(workOrder: workOrder),
        const SizedBox(height: 16),

        // ── Customer Info ──
        _InfoSection(
          title: 'Pelanggan',
          icon: Icons.person,
          children: [
            _InfoRow(
              label: 'Nama',
              value: workOrder.customerName ?? '-',
            ),
            if (workOrder.customerPhone != null)
              _InfoRow(
                label: 'Telepon',
                value: workOrder.customerPhone!,
              ),
            if (workOrder.locationLabel != null)
              _InfoRow(
                label: 'Alamat',
                value: workOrder.locationLabel!,
              ),
            if (workOrder.packageName != null)
              _InfoRow(
                label: 'Paket',
                value: workOrder.packageName!,
              ),
          ],
        ),
        const SizedBox(height: 16),

        // ── Contact & Maps ──
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Kontak & Lokasi',
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: () => _callCustomer(context),
                        icon: const Icon(Icons.phone),
                        label: const Text('Telepon'),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: () => _openWhatsApp(context),
                        icon: const Icon(Icons.chat),
                        label: const Text('WhatsApp'),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                SizedBox(
                  width: double.infinity,
                  child: OutlinedButton.icon(
                    onPressed: () => _openMaps(context),
                    icon: const Icon(Icons.map),
                    label: const Text('Buka di Google Maps'),
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),

        // ── Schedule ──
        if (workOrder.scheduledAt != null)
          _InfoSection(
            title: 'Jadwal',
            icon: Icons.calendar_today,
            children: [
              _InfoRow(
                label: 'Tanggal',
                value: workOrder.scheduledDateFormatted ?? '-',
              ),
              _InfoRow(
                label: 'Waktu',
                value: workOrder.scheduledTimeFormatted ?? '-',
              ),
            ],
          ),
        if (workOrder.scheduledAt != null) const SizedBox(height: 16),

        // ── Assignment ──
        if (workOrder.assignedToName != null)
          _InfoSection(
            title: 'Penugasan',
            icon: Icons.assignment_ind,
            children: [
              _InfoRow(
                label: 'Teknisi',
                value: workOrder.assignedToName ?? '-',
              ),
              if (workOrder.assignmentStatus != null)
                _InfoRow(
                  label: 'Status Assignment',
                  value: workOrder.assignmentStatus!,
                ),
            ],
          ),
        if (workOrder.assignedToName != null) const SizedBox(height: 16),

        // ── Notes ──
        if (workOrder.notes != null && workOrder.notes!.isNotEmpty)
          _InfoSection(
            title: 'Catatan',
            icon: Icons.note,
            children: [
              Text(
                workOrder.notes!,
                style: theme.textTheme.bodyMedium,
              ),
            ],
          ),
        if (workOrder.notes != null && workOrder.notes!.isNotEmpty)
          const SizedBox(height: 16),

        // ── Action Buttons ──
        _ActionButtons(workOrder: workOrder),
        const SizedBox(height: 32),
      ],
    );
  }

  void _callCustomer(BuildContext context) async {
    final phone = workOrder.customerPhone;
    if (phone == null || phone.isEmpty) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Nomor telepon pelanggan tidak tersedia')),
        );
      }
      return;
    }
    final url = Uri.parse('tel:$phone');
    if (await canLaunchUrl(url)) {
      await launchUrl(url);
    } else {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Tidak bisa membuka aplikasi telepon')),
        );
      }
    }
  }

  void _openWhatsApp(BuildContext context) async {
    final phone = workOrder.customerPhone;
    if (phone == null || phone.isEmpty) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Nomor WhatsApp pelanggan tidak tersedia')),
        );
      }
      return;
    }
    // Strip non-digits for WhatsApp
    final cleanPhone = phone.replaceAll(RegExp(r'[^\d]'), '');
    final url = Uri.parse('https://api.whatsapp.com/send?phone=$cleanPhone');
    if (await canLaunchUrl(url)) {
      await launchUrl(url, mode: LaunchMode.externalApplication);
    } else {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Tidak bisa membuka WhatsApp')),
        );
      }
    }
  }

  void _openMaps(BuildContext context) async {
    final lat = workOrder.locationLatitude;
    final lng = workOrder.locationLongitude;
    final Uri url;
    if (lat != null && lng != null) {
      // Use exact coordinates + label
      final label = Uri.encodeComponent(
        workOrder.locationLabel ?? 'Lokasi pelanggan',
      );
      url = Uri.parse(
        'https://www.google.com/maps?q=$lat,$lng',
      );
    } else {
      // Fallback to text search
      final query = Uri.encodeComponent(
        workOrder.locationLabel ?? 'Lokasi pelanggan',
      );
      url = Uri.parse(
        'https://www.google.com/maps/search/?api=1&query=$query',
      );
    }
    if (await canLaunchUrl(url)) {
      await launchUrl(url, mode: LaunchMode.externalApplication);
    } else {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Tidak bisa membuka Maps')),
        );
      }
    }
  }
}

// ── Status Card ──

class _StatusCard extends StatelessWidget {
  final WorkOrderModel workOrder;
  const _StatusCard({required this.workOrder});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    Color statusColor;
    IconData statusIcon;
    switch (workOrder.status) {
      case 'pending':
        statusColor = Colors.orange;
        statusIcon = Icons.schedule;
        break;
      case 'assigned':
        statusColor = Colors.blue;
        statusIcon = Icons.assignment_ind;
        break;
      case 'in_progress':
        statusColor = Colors.indigo;
        statusIcon = Icons.play_circle;
        break;
      case 'completed':
        statusColor = Colors.green;
        statusIcon = Icons.check_circle;
        break;
      case 'cancelled':
        statusColor = Colors.red;
        statusIcon = Icons.cancel;
        break;
      default:
        statusColor = Colors.grey;
        statusIcon = Icons.help_outline;
    }

    return Card(
      color: statusColor.withOpacity(0.1),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Icon(statusIcon, color: statusColor, size: 32),
            const SizedBox(width: 16),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    workOrder.statusLabel(),
                    style: theme.textTheme.titleLarge?.copyWith(
                      color: statusColor,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  Text(
                    'ID: ${workOrder.id}',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ── Info Section ──

class _InfoSection extends StatelessWidget {
  final String title;
  final IconData icon;
  final List<Widget> children;

  const _InfoSection({
    required this.title,
    required this.icon,
    required this.children,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(icon, size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text(
                  title,
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            ...children,
          ],
        ),
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  final String label;
  final String value;

  const _InfoRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 100,
            child: Text(
              label,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: theme.textTheme.bodyMedium,
            ),
          ),
        ],
      ),
    );
  }
}

// ── Action Buttons ──

class _ActionButtons extends ConsumerStatefulWidget {
  final WorkOrderModel workOrder;
  const _ActionButtons({required this.workOrder});

  @override
  ConsumerState<_ActionButtons> createState() => _ActionButtonsState();
}

class _ActionButtonsState extends ConsumerState<_ActionButtons> {
  bool _isLoading = false;

  Future<void> _performAction(String action) async {
    setState(() => _isLoading = true);

    final service = ref.read(workOrderServiceProvider);
    ServiceResult<WorkOrderModel> result;

    switch (action) {
      case 'claim':
        result = await service.claim(widget.workOrder.id);
        break;
      case 'start':
        result = await service.start(widget.workOrder.id);
        break;
      case 'complete':
        // Navigate to installation complete screen
        if (mounted) {
          context.push('/work-orders/${widget.workOrder.id}/complete');
        }
        setState(() => _isLoading = false);
        return;
      case 'cancel':
        result = await service.cancel(widget.workOrder.id);
        break;
      default:
        setState(() => _isLoading = false);
        return;
    }

    if (mounted) {
      switch (result) {
        case Success():
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Berhasil: $action'),
              backgroundColor: Colors.green,
            ),
          );
          ref.invalidate(workOrderDetailProvider(widget.workOrder.id));
        case Failure(:final exception):
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Gagal: ${exception.message}'),
              backgroundColor: Colors.red,
            ),
          );
      }
    }

    setState(() => _isLoading = false);
  }

  @override
  Widget build(BuildContext context) {
    final wo = widget.workOrder;
    final theme = Theme.of(context);

    List<Widget> buttons = [];

    if (wo.isPending) {
      buttons.add(
        SizedBox(
          width: double.infinity,
          child: FilledButton.icon(
            onPressed: _isLoading ? null : () => _performAction('claim'),
            icon: const Icon(Icons.assignment_ind),
            label: const Text('Ambil Work Order'),
          ),
        ),
      );
    }

    if (wo.isAssigned) {
      buttons.add(
        SizedBox(
          width: double.infinity,
          child: FilledButton.icon(
            onPressed: _isLoading ? null : () => _performAction('start'),
            icon: const Icon(Icons.play_arrow),
            label: const Text('Mulai Bekerja'),
          ),
        ),
      );
    }

    if (wo.isInProgress) {
      buttons.addAll([
        SizedBox(
          width: double.infinity,
          child: FilledButton.icon(
            onPressed: _isLoading ? null : () => _performAction('complete'),
            icon: const Icon(Icons.check),
            label: const Text('Selesaikan Instalasi'),
            style: FilledButton.styleFrom(
              backgroundColor: Colors.green,
            ),
          ),
        ),
        const SizedBox(height: 8),
        SizedBox(
          width: double.infinity,
          child: OutlinedButton.icon(
            onPressed: _isLoading ? null : () => _performAction('cancel'),
            icon: const Icon(Icons.cancel, color: Colors.red),
            label: const Text(
              'Batalkan',
              style: TextStyle(color: Colors.red),
            ),
          ),
        ),
      ]);
    }

    if (buttons.isEmpty) return const SizedBox.shrink();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (_isLoading)
          const Center(
            child: Padding(
              padding: EdgeInsets.all(16),
              child: CircularProgressIndicator(),
            ),
          )
        else
          ...buttons,
      ],
    );
  }
}
