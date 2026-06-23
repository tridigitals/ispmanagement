import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart' show workOrderServiceProvider;

class WorkOrderDetailScreen extends ConsumerStatefulWidget {
  const WorkOrderDetailScreen({super.key, required this.id});
  final String id;

  @override
  ConsumerState<WorkOrderDetailScreen> createState() =>
      _WorkOrderDetailScreenState();
}

class _WorkOrderDetailScreenState extends ConsumerState<WorkOrderDetailScreen> {
  WorkOrderModel? _wo;
  bool _loading = true;
  Object? _error;
  bool _actionLoading = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final svc = ref.read(workOrderServiceProvider);
      final result = await svc.getById(widget.id);
      final wo = result.getOrThrow();
      if (!mounted) return;
      setState(() {
        _wo = wo;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e;
        _loading = false;
      });
    }
  }

  Future<void> _performAction(
    Future<ServiceResult<WorkOrderModel>> Function(WorkOrderService) action,
    String successMsg,
  ) async {
    if (_actionLoading) return;
    setState(() => _actionLoading = true);
    try {
      final svc = ref.read(workOrderServiceProvider);
      final result = await action(svc);
      final updated = result.getOrThrow();
      if (!mounted) return;
      setState(() {
        _wo = updated;
        _actionLoading = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(successMsg),
          backgroundColor: context.isp.success,
        ),
      );
    } catch (e) {
      if (!mounted) return;
      setState(() => _actionLoading = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(e.toString()),
          backgroundColor: context.isp.danger,
        ),
      );
    }
  }

  Future<void> _completeWithNotes() async {
    final l10n = AppLocalizations.of(context);
    final notesCtrl = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.workOrderComplete),
        content: TextField(
          controller: notesCtrl,
          decoration: InputDecoration(
            hintText: l10n.workOrderNotesHint,
            border: const OutlineInputBorder(),
          ),
          maxLines: 3,
          minLines: 2,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(l10n.workOrderComplete),
          ),
        ],
      ),
    );
    if (ok == true && mounted) {
      await _performAction(
        (svc) => svc.complete(widget.id, notes: notesCtrl.text),
        l10n.workOrderConfirmed,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    if (_loading) {
      return Scaffold(
        appBar: AppBar(title: Text(l10n.workOrderDetail)),
        body: const Center(child: CircularProgressIndicator()),
      );
    }

    if (_error != null || _wo == null) {
      return Scaffold(
        appBar: AppBar(title: Text(l10n.workOrderDetail)),
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.error_outline, size: 48, color: isp.textMuted),
              const SizedBox(height: 12),
              Text(l10n.workOrderErrorLoad,
                  style: TextStyle(color: isp.textMuted)),
              const SizedBox(height: 12),
              ElevatedButton(
                onPressed: _load,
                child: Text(l10n.retry),
              ),
            ],
          ),
        ),
      );
    }

    final wo = _wo!;

    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.workOrderDetail),
      ),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            // Header: Status badge + ID
            Row(
              children: [
                _StatusBadge(status: wo.status),
                const SizedBox(width: 12),
                Text(
                  '#${wo.id}',
                  style: TextStyle(
                    fontSize: 13,
                    color: isp.textMuted,
                    fontFamily: 'monospace',
                  ),
                ),
                const Spacer(),
                if (wo.packageProvisioningType != null)
                  Container(
                    padding:
                        const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    decoration: BoxDecoration(
                      color: isp.accent.withOpacity(0.1),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      wo.packageProvisioningType!,
                      style: TextStyle(
                        fontSize: 11,
                        color: isp.accent,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 20),

            // Customer card
            _SectionCard(
              icon: Icons.person_outline,
              title: l10n.workOrderCustomer,
              children: [
                _InfoRow(label: l10n.fullName, value: wo.customerName),
                if (wo.customerPhone != null)
                  _InfoRow(label: l10n.phone, value: wo.customerPhone),
              ],
            ),
            const SizedBox(height: 12),

            // Location card
            if (wo.locationLabel != null)
              _SectionCard(
                icon: Icons.location_on_outlined,
                title: l10n.workOrderLocation,
                children: [
                  _InfoRow(label: l10n.location, value: wo.locationLabel),
                  if (wo.locationLatitude != null && wo.locationLongitude != null)
                    _InfoRow(
                      label: 'Coordinates',
                      value:
                          '${wo.locationLatitude!.toStringAsFixed(6)}, ${wo.locationLongitude!.toStringAsFixed(6)}',
                    ),
                ],
              ),

            // Package card
            _SectionCard(
              icon: Icons.inventory_2_outlined,
              title: l10n.workOrderPackage,
              children: [
                if (wo.packageName != null)
                  _InfoRow(label: l10n.internetPackage, value: wo.packageName),
                if (wo.routerName != null)
                  _InfoRow(label: l10n.workOrderRouter, value: wo.routerName),
              ],
            ),
            const SizedBox(height: 12),

            // Schedule card
            _SectionCard(
              icon: Icons.schedule,
              title: l10n.workOrderSchedule,
              children: [
                _InfoRow(
                  label: 'Created',
                  value: _formatDateTime(wo.createdAt),
                ),
                if (wo.scheduledAt != null)
                  _InfoRow(
                    label: 'Scheduled',
                    value: _formatDateTime(wo.scheduledAt!),
                  ),
                if (wo.completedAt != null)
                  _InfoRow(
                    label: 'Completed',
                    value: _formatDateTime(wo.completedAt!),
                  ),
                if (wo.assignedToName != null)
                  _InfoRow(
                    label: l10n.ticketAssignee,
                    value: wo.assignedToName,
                  ),
              ],
            ),
            const SizedBox(height: 12),

            // Notes
            if (wo.notes != null && wo.notes!.isNotEmpty) ...[
              _SectionCard(
                icon: Icons.notes,
                title: l10n.workOrderNotes,
                children: [
                  Text(
                    wo.notes!,
                    style: TextStyle(
                      fontSize: 14,
                      color: isp.textSecondary,
                      height: 1.5,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
            ],

            // Action buttons
            const SizedBox(height: 12),
            _buildActionButtons(wo),
          ],
        ),
      ),
    );
  }

  Widget _buildActionButtons(WorkOrderModel wo) {
    final isActive = wo.isActive;
    if (!isActive) return const SizedBox.shrink();

    final l10n = AppLocalizations.of(context);

    return _actionLoading
        ? const Center(
            child: Padding(
              padding: EdgeInsets.all(16),
              child: CircularProgressIndicator(),
            ),
          )
        : Column(
            children: [
              // Row 1: primary action (contextual)
              SizedBox(
                width: double.infinity,
                child: _primaryActionButton(wo, l10n),
              ),
              // Row 2: secondary actions
              if (wo.status != 'completed') ...[
                const SizedBox(height: 8),
                Row(
                  children: [
                    if (wo.status != 'cancelled')
                      Expanded(
                        child: OutlinedButton.icon(
                          onPressed: _actionLoading
                              ? null
                              : () => _performAction(
                                    (svc) => svc.cancel(widget.id),
                                    l10n.workOrderCancelled,
                                  ),
                          icon: const Icon(Icons.cancel_outlined, size: 18),
                          label: Text(l10n.workOrderCancel),
                        ),
                      ),
                  ],
                ),
              ],
            ],
          );
  }

  Widget _primaryActionButton(WorkOrderModel wo, AppLocalizations l10n) {
    switch (wo.status) {
      case 'pending':
        return FilledButton.icon(
          onPressed: _actionLoading
              ? null
              : () => _performAction(
                    (svc) => svc.claim(widget.id),
                    l10n.workOrderClaimed,
                  ),
          icon: const Icon(Icons.touch_app_outlined, size: 20),
          label: Text(l10n.workOrderClaim),
        );
      case 'assigned':
        return FilledButton.icon(
          onPressed: _actionLoading
              ? null
              : () => _performAction(
                    (svc) => svc.start(widget.id),
                    l10n.workOrderStarted,
                  ),
          icon: const Icon(Icons.play_arrow, size: 20),
          label: Text(l10n.workOrderStart),
          style: FilledButton.styleFrom(
            backgroundColor: context.isp.accent,
          ),
        );
      case 'in_progress':
        return FilledButton.icon(
          onPressed: _actionLoading ? null : _completeWithNotes,
          icon: const Icon(Icons.check_circle_outline, size: 20),
          label: Text(l10n.workOrderComplete),
          style: FilledButton.styleFrom(
            backgroundColor: context.isp.success,
          ),
        );
      default:
        return const SizedBox.shrink();
    }
  }

  String _formatDateTime(DateTime dt) {
    return '${dt.day}/${dt.month}/${dt.year} ${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }
}

// ─── Widgets ────────────────────────────────────────────────────

class _StatusBadge extends StatelessWidget {
  const _StatusBadge({required this.status});
  final String status;

  @override
  Widget build(BuildContext context) {
    final (label, color) = _statusInfo(status);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 12,
          fontWeight: FontWeight.w700,
          color: color,
        ),
      ),
    );
  }
}

(String, Color) _statusInfo(String status) {
  switch (status) {
    case 'pending':
      return ('Pending', Colors.grey);
    case 'assigned':
      return ('Assigned', Colors.orange);
    case 'in_progress':
      return ('In Progress', Colors.blue);
    case 'completed':
      return ('Completed', Colors.green);
    case 'cancelled':
      return ('Cancelled', Colors.red);
    default:
      return (status, Colors.grey);
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({
    required this.icon,
    required this.title,
    required this.children,
  });
  final IconData icon;
  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Container(
      decoration: BoxDecoration(
        color: isp.surface,
        borderRadius: BorderRadius.circular(IspRadii.md),
        border: Border.all(color: isp.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 14, 16, 0),
            child: Row(
              children: [
                Icon(icon, size: 16, color: isp.accent),
                const SizedBox(width: 8),
                Text(
                  title,
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w700,
                    color: isp.accent,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 10),
          ...children,
          const SizedBox(height: 8),
        ],
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});
  final String label;
  final String? value;

  @override
  Widget build(BuildContext context) {
    if (value == null) return const SizedBox.shrink();
    final isp = context.isp;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 90,
            child: Text(
              label,
              style: TextStyle(fontSize: 13, color: isp.textMuted),
            ),
          ),
          Expanded(
            child: Text(
              value!,
              style: TextStyle(fontSize: 13, color: isp.textPrimary),
            ),
          ),
        ],
      ),
    );
  }
}
