import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart' show workOrderServiceProvider, networkAssetServiceProvider;

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
  bool _creatingAsset = false;

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
    final nameCtrl = TextEditingController();
    final serialCtrl = TextEditingController();

    // Fetch terminal assets for this customer
    List<NetworkAssetListItemModel> assets = [];
    String? fetchError;
    try {
      final assetSvc = ref.read(networkAssetServiceProvider);
      final result = await assetSvc.listByCustomer(_wo!.customerId);
      assets = result.getOrThrow();
    } catch (e) {
      fetchError = e.toString();
    }

    if (!mounted) return;

    if (fetchError != null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Gagal memuat asset: $fetchError'),
          backgroundColor: context.isp.danger,
        ),
      );
      return;
    }

    NetworkAssetListItemModel? selectedAsset = assets.isNotEmpty ? assets.first : null;
    bool showCreateForm = assets.isEmpty;

    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setDialogState) {
          Future<void> createAsset() async {
            if (nameCtrl.text.trim().isEmpty) return;
            setDialogState(() => _creatingAsset = true);
            try {
              final assetSvc = ref.read(networkAssetServiceProvider);
              final result = await assetSvc.create(
                assetType: 'ont',
                name: nameCtrl.text.trim(),
                customerId: _wo!.customerId,
                serialNumber: serialCtrl.text.trim().isEmpty ? null : serialCtrl.text.trim(),
              );
              final created = result.getOrThrow();
              assets.add(created);
              selectedAsset = created;
              showCreateForm = false;
              nameCtrl.clear();
              serialCtrl.clear();
            } catch (e) {
              if (ctx.mounted) {
                ScaffoldMessenger.of(ctx).showSnackBar(
                  SnackBar(content: Text('Gagal: $e'), backgroundColor: context.isp.danger),
                );
              }
            } finally {
              setDialogState(() => _creatingAsset = false);
            }
          }

          return AlertDialog(
            title: Text(l10n.workOrderComplete),
            content: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  if (assets.isNotEmpty) ...[
                    Text(l10n.workOrderSelectTerminalAsset ?? 'Pilih terminal asset:', style: const TextStyle(fontWeight: FontWeight.w500)),
                    const SizedBox(height: 8),
                    Row(
                      children: [
                        Expanded(
                          child: Container(
                            decoration: BoxDecoration(
                              border: Border.all(color: context.isp.border),
                              borderRadius: BorderRadius.circular(8),
                            ),
                            padding: const EdgeInsets.symmetric(horizontal: 12),
                            child: DropdownButton<NetworkAssetListItemModel>(
                              value: selectedAsset,
                              isExpanded: true,
                              underline: const SizedBox.shrink(),
                              items: assets.map((a) => DropdownMenuItem(
                                value: a,
                                child: Text(a.displayLabel, style: const TextStyle(fontSize: 14)),
                              )).toList(),
                              onChanged: (v) => setDialogState(() => selectedAsset = v),
                            ),
                          ),
                        ),
                        IconButton(
                          icon: const Icon(Icons.add_circle_outline),
                          tooltip: 'Buat asset baru',
                          onPressed: () => setDialogState(() => showCreateForm = !showCreateForm),
                        ),
                      ],
                    ),
                  ],
                  if (showCreateForm) ...[
                    const SizedBox(height: 16),
                    const Divider(),
                    Text(assets.isEmpty ? 'Buat terminal asset baru:' : 'Atau buat baru:', style: const TextStyle(fontWeight: FontWeight.w500)),
                    const SizedBox(height: 8),
                    TextField(
                      controller: nameCtrl,
                      decoration: const InputDecoration(
                        labelText: 'Nama ONT/ONU *',
                        hintText: 'e.g. ONT-HG8245H',
                        border: OutlineInputBorder(),
                        isDense: true,
                      ),
                    ),
                    const SizedBox(height: 8),
                    TextField(
                      controller: serialCtrl,
                      decoration: InputDecoration(
                        labelText: 'Serial Number',
                        hintText: 'e.g. 485754431234ABCD',
                        border: const OutlineInputBorder(),
                        isDense: true,
                        suffixIcon: IconButton(
                          icon: const Icon(Icons.qr_code_scanner),
                          tooltip: 'Scan Barcode',
                          onPressed: () async {
                            final scanned = await Navigator.push<String>(
                              ctx,
                              MaterialPageRoute(builder: (_) => const _BarcodeScanPage()),
                            );
                            if (scanned != null && scanned.isNotEmpty) {
                              serialCtrl.text = scanned;
                              setDialogState(() {});
                            }
                          },
                        ),
                      ),
                    ),
                    const SizedBox(height: 8),
                    SizedBox(
                      width: double.infinity,
                      child: OutlinedButton.icon(
                        onPressed: _creatingAsset ? null : createAsset,
                        icon: _creatingAsset
                            ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))
                            : const Icon(Icons.add),
                        label: const Text('Simpan Asset'),
                      ),
                    ),
                  ],
                  const SizedBox(height: 16),
                  const Divider(),
                  Text(l10n.workOrderNotesHint ?? 'Catatan (opsional):', style: const TextStyle(fontWeight: FontWeight.w500)),
                  const SizedBox(height: 8),
                  TextField(
                    controller: notesCtrl,
                    decoration: InputDecoration(
                      hintText: l10n.workOrderNotesHint,
                      border: const OutlineInputBorder(),
                    ),
                    maxLines: 3,
                    minLines: 2,
                  ),
                ],
              ),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(ctx, false),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: selectedAsset == null ? null : () => Navigator.pop(ctx, true),
                child: Text(l10n.workOrderComplete),
              ),
            ],
          );
        },
      ),
    );
    if (ok == true && mounted && selectedAsset != null) {
      await _performAction(
        (svc) => svc.complete(
          widget.id,
          notes: notesCtrl.text.isEmpty ? null : notesCtrl.text,
          terminalAssetId: selectedAsset!.id,
        ),
        l10n.workOrderConfirmed,
      );
    }
  }

  // ─── Contact Actions ──────────────────────────────────────────

  void _openPhone(String phone) {
    launchUrl(Uri(scheme: 'tel', path: phone.replaceAll(RegExp(r'[^\d+]'), '')));
  }

  void _openWhatsApp(String phone) {
    final cleaned = phone.replaceAll(RegExp(r'[^\d+]'), '');
    final msg = Uri.encodeComponent(
        'Halo, saya teknisi ISP. Terkait pemasangan di ${_wo?.locationLabel ?? 'lokasi Anda'}...');
    launchUrl(
        Uri.parse('https://wa.me/$cleaned?text=$msg'));
  }

  void _openMaps() {
    final wo = _wo;
    if (wo == null) return;
    if (wo.locationLatitude != null && wo.locationLongitude != null) {
      launchUrl(Uri.parse(
          'https://www.google.com/maps/dir/?api=1&destination=${wo.locationLatitude},${wo.locationLongitude}'));
    } else if (wo.locationLabel != null) {
      launchUrl(Uri.parse(
          'https://www.google.com/maps/search/?api=1&query=${Uri.encodeComponent(wo.locationLabel!)}'));
    }
  }

  // ─── Step Indicator ───────────────────────────────────────────

  Widget _buildStepIndicator(WorkOrderModel wo, AppLocalizations l10n) {
    final steps = _stepsForStatus(wo.status);

    // Determine active step index
    int activeIdx;
    switch (wo.status) {
      case 'pending':
        activeIdx = 0;
        break;
      case 'assigned':
        activeIdx = 1;
        break;
      case 'in_progress':
        activeIdx = 2;
        break;
      case 'completed':
        activeIdx = 3;
        break;
      case 'cancelled':
        return const SizedBox.shrink();
      default:
        activeIdx = 0;
    }

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 12),
      child: Row(
        children: List.generate(steps.length * 2 - 1, (i) {
          if (i.isOdd) {
            // Connector line
            final lineActive = (i ~/ 2) < activeIdx;
            return Expanded(
              child: Container(
                height: 3,
                margin: const EdgeInsets.symmetric(horizontal: 4),
                decoration: BoxDecoration(
                  color: lineActive ? context.isp.accent : context.isp.border,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            );
          }
          // Step circle
          final stepIdx = i ~/ 2;
          final isCompleted = stepIdx < activeIdx;
          final isActive = stepIdx == activeIdx &&
              wo.status != 'completed' &&
              wo.status != 'cancelled';
          final stepLabel = steps[stepIdx];

          return Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 32,
                height: 32,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: isCompleted || isActive
                      ? isCompleted
                          ? context.isp.success
                          : context.isp.accent
                      : context.isp.surface,
                  border: Border.all(
                    color: isCompleted || isActive
                        ? context.isp.accent
                        : context.isp.border,
                    width: 2,
                  ),
                ),
                child: Center(
                  child: isCompleted
                      ? const Icon(Icons.check, size: 16, color: Colors.white)
                      : Text(
                          '${stepIdx + 1}',
                          style: TextStyle(
                            fontSize: 13,
                            fontWeight: FontWeight.w700,
                            color: isActive
                                ? Colors.white
                                : context.isp.textMuted,
                          ),
                        ),
                ),
              ),
              const SizedBox(height: 4),
              Text(
                stepLabel,
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: isActive ? FontWeight.w600 : FontWeight.w400,
                  color:
                      isActive || isCompleted ? context.isp.accent : context.isp.textMuted,
                ),
              ),
            ],
          );
        }),
      ),
    );
  }

  List<String> _stepsForStatus(String status) {
    if (status == 'cancelled') return [];
    return [
      AppLocalizations.of(context).workOrderStepClaim,
      AppLocalizations.of(context).workOrderStepStart,
      AppLocalizations.of(context).workOrderStepComplete,
    ];
  }

  // ─── Contact Buttons ──────────────────────────────────────────

  Widget _buildContactButtons(WorkOrderModel wo) {
    final phone = wo.customerPhone;
    final hasPhone = phone != null && phone.isNotEmpty;
    final isp = context.isp;

    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        _ContactIcon(
          icon: Icons.phone,
          color: Colors.green,
          onTap: hasPhone ? () => _openPhone(phone!) : null,
        ),
        const SizedBox(width: 24),
        _ContactIcon(
          icon: Icons.chat,
          color: const Color(0xFF25D366),
          onTap: hasPhone ? () => _openWhatsApp(phone!) : null,
        ),
        const SizedBox(width: 24),
        _ContactIcon(
          icon: Icons.map,
          color: isp.accent,
          onTap: _openMaps,
        ),
      ],
    );
  }

  Widget _buildActionButton(WorkOrderModel wo, AppLocalizations l10n) {
    final isActive = wo.isActive;
    if (!isActive) return const SizedBox.shrink();

    if (_actionLoading) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.all(16),
          child: CircularProgressIndicator(),
        ),
      );
    }

    switch (wo.status) {
      case 'pending':
        return SizedBox(
          width: double.infinity,
          child: FilledButton.icon(
            onPressed: () => _performAction(
              (svc) => svc.claim(widget.id),
              l10n.workOrderClaimed,
            ),
            icon: const Icon(Icons.touch_app_outlined, size: 20),
            label: Text(l10n.workOrderClaim),
          ),
        );
      case 'assigned':
        return SizedBox(
          width: double.infinity,
          child: FilledButton.icon(
            onPressed: () => _performAction(
              (svc) => svc.start(widget.id),
              l10n.workOrderStarted,
            ),
            icon: const Icon(Icons.play_arrow, size: 20),
            label: Text(l10n.workOrderStart),
            style: FilledButton.styleFrom(
              backgroundColor: context.isp.accent,
            ),
          ),
        );
      case 'in_progress':
        return SizedBox(
          width: double.infinity,
          child: FilledButton.icon(
            onPressed: _completeWithNotes,
            icon: const Icon(Icons.check_circle_outline, size: 20),
            label: Text(l10n.workOrderComplete),
            style: FilledButton.styleFrom(
              backgroundColor: context.isp.success,
            ),
          ),
        );
      default:
        return const SizedBox.shrink();
    }
  }

  Widget _buildCancelButton(WorkOrderModel wo, AppLocalizations l10n) {
    if (!wo.isActive || wo.status == 'completed') return const SizedBox.shrink();
    return SizedBox(
      width: double.infinity,
      child: TextButton.icon(
        onPressed: _actionLoading
            ? null
            : () => _performAction(
                  (svc) => svc.cancel(widget.id),
                  l10n.workOrderCancelled,
                ),
        icon: const Icon(Icons.cancel_outlined, size: 18),
        label: Text(l10n.workOrderCancel),
        style: TextButton.styleFrom(foregroundColor: context.isp.danger),
      ),
    );
  }

  // ─── Build ────────────────────────────────────────────────────

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
      appBar: AppBar(title: Text(l10n.workOrderDetail)),
      body: SafeArea(
        child: Column(
          children: [
            // Scrollable content
            Expanded(
              child: ListView(
                padding: const EdgeInsets.all(16),
                children: [
                  // Header: Status badge + ID + type
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
                          padding: const EdgeInsets.symmetric(
                              horizontal: 8, vertical: 2),
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
                  const SizedBox(height: 4),

                  // ── Step Indicator ──
                  _buildStepIndicator(wo, l10n),
                  const SizedBox(height: 16),

                  // ── Customer card ──
                  _SectionCard(
                    icon: Icons.person_outline,
                    title: l10n.workOrderCustomer,
                    children: [
                      _InfoRow(label: l10n.fullName, value: wo.customerName),
                      if (wo.customerPhone != null)
                        _InfoRow(label: l10n.phone, value: wo.customerPhone),
                      if (wo.customerPhone == null)
                        _InfoRow(
                            label: l10n.phone, value: l10n.noPhoneNumber),
                    ],
                  ),
                  const SizedBox(height: 12),

                  // ── Location card ──
                  if (wo.locationLabel != null)
                    _SectionCard(
                      icon: Icons.location_on_outlined,
                      title: l10n.workOrderLocation,
                      children: [
                        _InfoRow(label: l10n.location, value: wo.locationLabel),
                        if (wo.locationLatitude != null &&
                            wo.locationLongitude != null)
                          _InfoRow(
                            label: 'Coordinates',
                            value:
                                '${wo.locationLatitude!.toStringAsFixed(6)}, ${wo.locationLongitude!.toStringAsFixed(6)}',
                          ),
                      ],
                    ),
                  const SizedBox(height: 12),

                  // ── Package card ──
                  _SectionCard(
                    icon: Icons.inventory_2_outlined,
                    title: l10n.workOrderPackage,
                    children: [
                      if (wo.packageName != null)
                        _InfoRow(
                            label: l10n.internetPackage,
                            value: wo.packageName),
                      if (wo.routerName != null)
                        _InfoRow(
                            label: l10n.workOrderRouter, value: wo.routerName),
                    ],
                  ),
                  const SizedBox(height: 12),

                  // ── Schedule card ──
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

                  // ── Notes ──
                  if (wo.notes != null && wo.notes!.isNotEmpty) ...[
                    const SizedBox(height: 12),
                    _SectionCard(
                      icon: Icons.notes,
                      title: l10n.workOrderNotes,
                      children: [
                        Padding(
                          padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
                          child: Text(
                            wo.notes!,
                            style: TextStyle(
                              fontSize: 14,
                              color: isp.textSecondary,
                              height: 1.5,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ],

                  const SizedBox(height: 20),
                ],
              ),
            ),

            // ── Sticky bottom: Contact buttons + Action ──
            Container(
              decoration: BoxDecoration(
                color: isp.surface,
                border: Border(top: BorderSide(color: isp.border)),
              ),
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 16),
              child: SafeArea(
                top: false,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    // Contact quick-actions
                    if (wo.isActive) ...[
                      _buildContactButtons(wo),
                      const SizedBox(height: 12),
                    ],
                    // Primary action button
                    _buildActionButton(wo, l10n),
                    // Cancel (subtle)
                    if (wo.isActive && wo.status != 'completed') ...[
                      const SizedBox(height: 6),
                      _buildCancelButton(wo, l10n),
                    ],
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  String _formatDateTime(DateTime dt) {
    return '${dt.day}/${dt.month}/${dt.year} ${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }
}

// ─── Contact Icon ───────────────────────────────────────────────

class _ContactIcon extends StatelessWidget {
  const _ContactIcon({
    required this.icon,
    required this.color,
    this.onTap,
  });
  final IconData icon;
  final Color color;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final disabled = onTap == null;
    return GestureDetector(
      onTap: onTap,
      child: Container(
        width: 48,
        height: 48,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: disabled
              ? context.isp.border
              : color.withOpacity(0.12),
          border: Border.all(
            color: disabled ? context.isp.borderSubtle : color.withOpacity(0.3),
          ),
        ),
        child: Icon(
          icon,
          size: 22,
          color: disabled ? context.isp.textMuted : color,
        ),
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
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        label,
        style: TextStyle(fontSize: 12, fontWeight: FontWeight.w700, color: color),
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

// ─── Section Card ───────────────────────────────────────────────

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

/// Full-screen barcode scanner for ONT serial numbers.
class _BarcodeScanPage extends StatefulWidget {
  const _BarcodeScanPage();

  @override
  State<_BarcodeScanPage> createState() => _BarcodeScanPageState();
}

class _BarcodeScanPageState extends State<_BarcodeScanPage> {
  MobileScannerController? _controller;
  bool _scanned = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _initCamera();
  }

  Future<void> _initCamera() async {
    try {
      _controller = MobileScannerController(
        detectionSpeed: DetectionSpeed.normal,
        facing: CameraFacing.back,
        autoStart: true,
      );
      await _controller!.start();
      if (mounted) setState(() {});
    } catch (e) {
      if (mounted) setState(() => _error = e.toString());
    }
  }

  @override
  void dispose() {
    _controller?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Scan Serial Number')),
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.camera_alt, size: 64, color: Colors.grey),
                const SizedBox(height: 16),
                Text('Tidak dapat mengakses kamera:\n$_error',
                    textAlign: TextAlign.center),
                const SizedBox(height: 16),
                ElevatedButton(
                  onPressed: () {
                    setState(() => _error = null);
                    _initCamera();
                  },
                  child: const Text('Coba Lagi'),
                ),
              ],
            ),
          ),
        ),
      );
    }

    if (_controller == null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Scan Serial Number')),
        body: const Center(child: CircularProgressIndicator()),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: const Text('Scan Serial Number'),
        actions: [
          IconButton(
            icon: ValueListenableBuilder(
              valueListenable: _controller!,
              builder: (_, state, __) => Icon(
                state.torchState == TorchState.on ? Icons.flash_on : Icons.flash_off,
              ),
            ),
            onPressed: () => _controller!.toggleTorch(),
          ),
        ],
      ),
      body: Stack(
        children: [
          MobileScanner(
            controller: _controller!,
            errorBuilder: (_, error, __) => Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.error, size: 64, color: Colors.red),
                    const SizedBox(height: 16),
                    Text('Kamera error: ${error.errorDetails?.message ?? error.errorCode}',
                        textAlign: TextAlign.center),
                  ],
                ),
              ),
            ),
            onDetect: (capture) {
              if (_scanned) return;
              final barcode = capture.barcodes.firstOrNull;
              if (barcode?.rawValue != null && barcode!.rawValue!.isNotEmpty) {
                _scanned = true;
                Navigator.pop(context, barcode.rawValue);
              }
            },
          ),
          Center(
            child: Container(
              width: 280,
              height: 140,
              decoration: BoxDecoration(
                border: Border.all(color: Colors.greenAccent, width: 2),
                borderRadius: BorderRadius.circular(12),
              ),
            ),
          ),
          Positioned(
            bottom: 48,
            left: 0,
            right: 0,
            child: Center(
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                decoration: BoxDecoration(
                  color: Colors.black54,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: const Text(
                  'Arahkan kamera ke barcode ONT',
                  style: TextStyle(color: Colors.white, fontSize: 14),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
