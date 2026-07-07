import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';
import '../../services/settings_providers.dart' show currentTabProvider;

// ─── Neubrutalist card ───────────────────────────────────────────

BoxDecoration _nbCard(IspThemeColors isp) => BoxDecoration(
      color: isp.surface,
      borderRadius: BorderRadius.circular(20),
      border: Border.all(color: isp.border, width: 1.5),
      boxShadow: [
        BoxShadow(
          color: isp.border.withOpacity(0.5),
          offset: const Offset(3, 3),
          blurRadius: 0,
        ),
      ],
    );

// ─── Status pill helper ──────────────────────────────────────────

Widget _statusPill(IspThemeColors isp, String label, Color color) {
  return Container(
    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
    decoration: BoxDecoration(
      color: color.withOpacity(0.15),
      borderRadius: BorderRadius.circular(999),
      border: Border.all(color: color.withOpacity(0.3), width: 1),
    ),
    child: Text(
      label,
      style: TextStyle(fontSize: 11, fontWeight: FontWeight.w700, color: color),
    ),
  );
}

// ─── Filter state ────────────────────────────────────────────────

enum _InvFilter { all, unpaid, paid }

class InvoicesTab extends ConsumerStatefulWidget {
  const InvoicesTab({super.key});

  @override
  ConsumerState<InvoicesTab> createState() => _InvoicesTabState();
}

class _InvoicesTabState extends ConsumerState<InvoicesTab> {
  final List<InvoiceModel> _items = [];
  int _page = 1;
  bool _hasMore = true;
  bool _loadingMore = false;
  bool _initialLoaded = false;
  Object? _initialError;
  _InvFilter _filter = _InvFilter.all;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadInitial());
    ref.listen(currentTabProvider, (prev, next) {
      if (next == 2 && prev != next) _loadInitial();
    });
  }

  Future<void> _loadInitial() async {
    try {
      final svc = ref.read(invoiceServiceProvider);
      final result = await svc.list(page: 1, perPage: 20);
      final paginated = result.getOrThrow();
      if (!mounted) return;
      setState(() {
        _items..clear()..addAll(paginated.data);
        _hasMore = paginated.hasMore;
        _page = 1;
        _initialLoaded = true;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() { _initialError = e; _initialLoaded = true; });
    }
  }

  Future<void> _loadMore() async {
    if (_loadingMore || !_hasMore) return;
    setState(() => _loadingMore = true);
    try {
      final svc = ref.read(invoiceServiceProvider);
      final result = await svc.list(page: _page + 1, perPage: 20);
      final paginated = result.getOrThrow();
      if (!mounted) return;
      setState(() {
        _items.addAll(paginated.data);
        _hasMore = paginated.hasMore;
        _page++;
        _loadingMore = false;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() => _loadingMore = false);
    }
  }

  List<InvoiceModel> get _filtered {
    switch (_filter) {
      case _InvFilter.unpaid:
        return _items.where((i) => !i.isPaid).toList();
      case _InvFilter.paid:
        return _items.where((i) => i.isPaid).toList();
      default:
        return _items;
    }
  }

  void _refreshForTabActivation() {
    setState(() { _items.clear(); _page = 1; _hasMore = true; _initialLoaded = false; _initialError = null; });
    _loadInitial();
  }

  bool _onScroll(Notification notification) {
    if (notification is ScrollNotification &&
        notification.metrics.extentAfter < notification.metrics.maxScrollExtent * 0.1) {
      _loadMore();
    }
    return false;
  }

  @override
  Widget build(BuildContext context) {
    ref.listen(currentTabProvider, (prev, next) {
      if (next == 2 && prev != next) _refreshForTabActivation();
    });

    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    if (!_initialLoaded) {
      return Center(child: CircularProgressIndicator(color: isp.accent));
    }

    if (_initialError != null) {
      return Scaffold(
        body: Center(
          child: Column(mainAxisSize: MainAxisSize.min, children: [
            Icon(Icons.error_outline, size: 48, color: isp.danger),
            const SizedBox(height: 12),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 24),
              child: Text(_initialError.toString(), textAlign: TextAlign.center, style: TextStyle(color: isp.textSecondary)),
            ),
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: () { setState(() { _initialLoaded = false; _initialError = null; }); _loadInitial(); },
              icon: const Icon(Icons.refresh),
              label: Text(l10n.retry),
            ),
          ]),
        ),
      );
    }

    if (_items.isEmpty) {
      return Scaffold(
        body: Center(
          child: Column(mainAxisSize: MainAxisSize.min, children: [
            Icon(Icons.receipt_long_outlined, size: 64, color: isp.textMuted),
            const SizedBox(height: 12),
            Text(l10n.noInvoicesYet, style: TextStyle(color: isp.textMuted)),
          ]),
        ),
      );
    }

    final filtered = _filtered;

    return NotificationListener<ScrollNotification>(
      onNotification: _onScroll,
      child: RefreshIndicator(
        color: isp.accent,
        onRefresh: () async {
          setState(() { _items.clear(); _page = 1; _hasMore = true; _initialLoaded = false; });
          await _loadInitial();
        },
        child: CustomScrollView(
          slivers: [
            // ── Filter pills ──
            SliverToBoxAdapter(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
                child: Row(children: [
                  _FilterPill(label: 'Semua', selected: _filter == _InvFilter.all, onTap: () => setState(() => _filter = _InvFilter.all)),
                  const SizedBox(width: 6),
                  _FilterPill(label: 'Belum Bayar', selected: _filter == _InvFilter.unpaid, onTap: () => setState(() => _filter = _InvFilter.unpaid)),
                  const SizedBox(width: 6),
                  _FilterPill(label: 'Lunas', selected: _filter == _InvFilter.paid, onTap: () => setState(() => _filter = _InvFilter.paid)),
                ]),
              ),
            ),
            // ── Invoice list ──
            SliverList(
              delegate: SliverChildBuilderDelegate(
                (context, index) {
                  if (index == filtered.length) {
                    return _loadingMore
                        ? const Padding(padding: EdgeInsets.all(24), child: Center(child: SizedBox(width: 24, height: 24, child: CircularProgressIndicator(strokeWidth: 2))))
                        : const SizedBox.shrink();
                  }
                  return _InvoiceTile(inv: filtered[index]);
                },
                childCount: filtered.length + 1,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ─── Filter pill ─────────────────────────────────────────────────

class _FilterPill extends StatelessWidget {
  const _FilterPill({required this.label, required this.selected, required this.onTap});
  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        decoration: BoxDecoration(
          color: selected ? isp.accent : isp.surface,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(color: selected ? isp.accent : isp.border, width: 1.5),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 11,
            fontWeight: FontWeight.w700,
            color: selected ? Colors.white : isp.textSecondary,
          ),
        ),
      ),
    );
  }
}

// ─── Invoice tile ────────────────────────────────────────────────

class _InvoiceTile extends StatelessWidget {
  const _InvoiceTile({required this.inv});
  final InvoiceModel inv;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final fmt = NumberFormat.simpleCurrency(name: inv.currencyCode);
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');
    final isPaid = inv.isPaid;
    final statusColor = isPaid ? isp.success : inv.isOverdue ? isp.danger : isp.warning;
    final dateStr = inv.dueDate != null ? dateFmt.format(inv.dueDate!) : '-';

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 5),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () => GoRouter.of(context).push('/invoices/${inv.id}'),
          borderRadius: BorderRadius.circular(16),
          child: Container(
            decoration: _nbCard(isp),
            padding: const EdgeInsets.all(14),
            child: Row(children: [
              // Left: content
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text(
                    inv.invoiceNumber ?? inv.id,
                    style: TextStyle(fontSize: 14, fontWeight: FontWeight.w700, color: isp.textPrimary),
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 2),
                  Text(
                    '$dateStr',
                    style: TextStyle(fontSize: 11, color: isp.textMuted),
                  ),
                ]),
              ),
              // Right: amount + status
              Column(crossAxisAlignment: CrossAxisAlignment.end, children: [
                Text(
                  fmt.format(inv.amount),
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.w800, color: isp.textPrimary, letterSpacing: -0.5),
                ),
                const SizedBox(height: 4),
                _statusPill(isp, inv.statusLabel(), statusColor),
              ]),
            ]),
          ),
        ),
      ),
    );
  }
}
