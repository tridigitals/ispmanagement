import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart' show workOrderServiceProvider;
import '../../services/settings_providers.dart' show currentTabProvider;

class WorkOrdersTab extends ConsumerStatefulWidget {
  const WorkOrdersTab({super.key});

  @override
  ConsumerState<WorkOrdersTab> createState() => _WorkOrdersTabState();
}

class _WorkOrdersTabState extends ConsumerState<WorkOrdersTab> {
  final List<WorkOrderModel> _items = [];
  bool _initialLoaded = false;
  bool _loading = false;
  Object? _error;
  String? _statusFilter; // null = all, 'pending', 'assigned', 'in_progress', 'completed'

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
    // Reload data when this tab becomes active (IndexedStack keeps all tabs alive)
    ref.listen(currentTabProvider, (prev, next) {
      if (next == 2 && prev != next) {
        _load();
      }
    });
  }

  Future<void> _load() async {
    if (_loading) return;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final svc = ref.read(workOrderServiceProvider);
      final result = await svc.list(
        status: _statusFilter,
        includeClosed: _statusFilter == 'completed',
        limit: 200,
      );
      final data = result.getOrThrow();
      if (!mounted) return;
      setState(() {
        _items
          ..clear()
          ..addAll(data);
        _initialLoaded = true;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e;
        _initialLoaded = true;
        _loading = false;
      });
    }
  }

  void _onFilterChanged(String? status) {
    if (status == _statusFilter) return;
    setState(() {
      _statusFilter = status;
      _initialLoaded = false;
      _items.clear();
    });
    _load();
  }

  void _refreshForTabActivation() {
    setState(() {
      _items.clear();
      _initialLoaded = false;
      _error = null;
      _statusFilter = null;
    });
    _load();
  }

  Widget _buildFilterChips(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final filters = <(String, String?)>[
      (l10n.workOrderTabAll, null),
      (l10n.workOrderTabPending, 'pending'),
      (l10n.workOrderStatusAssigned, 'assigned'),
      (l10n.workOrderTabInProgress, 'in_progress'),
      (l10n.workOrderTabCompleted, 'completed'),
    ];

    return SizedBox(
      height: 44,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 16),
        itemCount: filters.length,
        separatorBuilder: (_, __) => const SizedBox(width: 8),
        itemBuilder: (_, i) {
          final (label, status) = filters[i];
          final selected = _statusFilter == status;
          return FilterChip(
            label: Text(label),
            selected: selected,
            onSelected: (_) => _onFilterChanged(status),
            selectedColor: isp.accent.withOpacity(0.15),
            checkmarkColor: isp.accent,
            labelStyle: TextStyle(
              fontSize: 13,
              color: selected ? isp.accent : isp.textSecondary,
              fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
            ),
            side: BorderSide(
              color: selected ? isp.accent : isp.border,
            ),
            backgroundColor: isp.surface,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(20),
            ),
          );
        },
      ),
    );
  }

  Widget _buildTile(BuildContext context, WorkOrderModel wo) {
    final isp = context.isp;

    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: () => GoRouter.of(context).push('/work-orders/${wo.id}'),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          decoration: BoxDecoration(
            border: Border(
              bottom: BorderSide(color: isp.border, width: 1.5),
            ),
          ),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Status color stripe
              Container(
                width: 4,
                height: 56,
                decoration: BoxDecoration(
                  color: _statusColor(wo.status),
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const SizedBox(width: 12),
              // Content
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // Customer name + WO ID
                    Text(
                      wo.customerName ?? 'Pelanggan #${wo.customerId.substring(0, 8)}',
                      style: TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w600,
                        color: isp.textPrimary,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 4),
                    // Package + location
                    if (wo.packageName != null)
                      Row(
                        children: [
                          Icon(Icons.inventory_2_outlined,
                              size: 13, color: isp.textMuted),
                          const SizedBox(width: 4),
                          Expanded(
                            child: Text(
                              wo.packageName!,
                              style: TextStyle(
                                  fontSize: 12, color: isp.textSecondary),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ],
                      ),
                    if (wo.locationLabel != null) ...[
                      const SizedBox(height: 2),
                      Row(
                        children: [
                          Icon(Icons.location_on_outlined,
                              size: 13, color: isp.textMuted),
                          const SizedBox(width: 4),
                          Expanded(
                            child: Text(
                              wo.locationLabel!,
                              style: TextStyle(
                                  fontSize: 12, color: isp.textSecondary),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ],
                      ),
                    ],
                    const SizedBox(height: 6),
                    // Status badge + ID
                    Row(
                      children: [
                        _StatusBadge(status: wo.status),
                        const SizedBox(width: 8),
                        Text(
                          '#${wo.id.substring(0, 8)}',
                          style: TextStyle(
                              fontSize: 11, color: isp.textMuted),
                        ),
                        if (wo.scheduledAt != null) ...[
                          const SizedBox(width: 8),
                          Icon(Icons.schedule,
                              size: 12, color: isp.textMuted),
                          const SizedBox(width: 3),
                          Text(
                            _formatSchedule(wo.scheduledAt!),
                            style: TextStyle(
                                fontSize: 11, color: isp.textMuted),
                          ),
                        ],
                      ],
                    ),
                  ],
                ),
              ),
              const Icon(Icons.chevron_right, color: Colors.grey, size: 20),
            ],
          ),
        ),
      ),
    );
  }

  Color _statusColor(String status) {
    switch (status) {
      case 'pending':
        return Colors.grey;
      case 'assigned':
        return Colors.orange;
      case 'in_progress':
        return Colors.blue;
      case 'completed':
        return Colors.green;
      case 'cancelled':
        return Colors.red.shade300;
      default:
        return Colors.grey;
    }
  }

  String _formatSchedule(DateTime dt) {
    return '${dt.day}/${dt.month} ${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    // Reload data when this tab becomes active (IndexedStack keeps all tabs alive)
    ref.listen(currentTabProvider, (prev, next) {
      if (next == 2 && prev != next) {
        _refreshForTabActivation();
      }
    });

    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Column(
      children: [
        const SizedBox(height: 8),
        // Sticky filter chips
        _buildFilterChips(context),
        const SizedBox(height: 8),
        // Body — loading / error / empty / list
        Expanded(
          child: _buildBody(context),
        ),
      ],
    );
  }

  Widget _buildBody(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    if (!_initialLoaded) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_error != null && _items.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.error_outline, size: 48, color: isp.textMuted),
              const SizedBox(height: 12),
              Text(
                l10n.workOrderErrorLoad,
                style: TextStyle(color: isp.textMuted),
              ),
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

    return RefreshIndicator(
      onRefresh: () async {
        _onFilterChanged(_statusFilter);
      },
      color: isp.accent,
      child: _items.isEmpty
          ? ListView(
              children: [
                SizedBox(
                    height: MediaQuery.of(context).size.height * 0.3),
                Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.assignment_outlined,
                          size: 48, color: isp.textMuted),
                      const SizedBox(height: 8),
                      Text(
                        l10n.workOrderNoAssigned,
                        style: TextStyle(
                          color: isp.textMuted,
                          fontSize: 16,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            )
          : ListView.builder(
              itemCount: _items.length,
              itemBuilder: (_, i) =>
                  _buildTile(context, _items[i]),
            ),
    );
  }
}

// ─── Status Badge ───────────────────────────────────────────────

class _StatusBadge extends StatelessWidget {
  const _StatusBadge({required this.status});
  final String status;

  @override
  Widget build(BuildContext context) {
    final (label, color) = _statusInfo(status);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 10,
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
