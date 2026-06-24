import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';
import '../../services/settings_providers.dart' show currentTabProvider;
import '../../theme/app_theme.dart';

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

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadInitial());
    // Reload data when this tab becomes active (IndexedStack keeps all tabs alive)
    ref.listen(currentTabProvider, (prev, next) {
      if (next == 2 && prev != next) {
        _loadInitial();
      }
    });
  }

  Future<void> _loadInitial() async {
    try {
      final svc = ref.read(invoiceServiceProvider);
      final result = await svc.list(page: 1, perPage: 20);
      final paginated = result.getOrThrow();
      if (!mounted) return;
      setState(() {
        _items
          ..clear()
          ..addAll(paginated.data);
        _hasMore = paginated.hasMore;
        _page = 1;
        _initialLoaded = true;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _initialError = e;
        _initialLoaded = true;
      });
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

  void _refreshForTabActivation() {
    setState(() {
      _items.clear();
      _page = 1;
      _hasMore = true;
      _initialLoaded = false;
      _initialError = null;
    });
    _loadInitial();
  }

  bool _onScroll(Notification notification) {
    if (notification is ScrollNotification &&
        notification.metrics.extentAfter <
            notification.metrics.maxScrollExtent * 0.1) {
      _loadMore();
    }
    return false;
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

    if (!_initialLoaded) {
      return Center(
        child: CircularProgressIndicator(color: isp.accent),
      );
    }

    if (_initialError != null) {
      return Scaffold(
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.error_outline, size: 48, color: isp.danger),
              const SizedBox(height: 12),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 24),
                child: Text(
                  _initialError.toString(),
                  textAlign: TextAlign.center,
                  style: TextStyle(color: isp.textSecondary),
                ),
              ),
              const SizedBox(height: 16),
              OutlinedButton.icon(
                onPressed: () {
                  setState(() {
                    _initialLoaded = false;
                    _initialError = null;
                  });
                  _loadInitial();
                },
                icon: const Icon(Icons.refresh),
                label: Text(l10n.retry),
              ),
            ],
          ),
        ),
      );
    }

    if (_items.isEmpty) {
      return Scaffold(
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.receipt_long_outlined,
                size: 64,
                color: isp.textMuted,
              ),
              const SizedBox(height: 12),
              Text(
                l10n.noInvoicesYet,
                style: TextStyle(color: isp.textMuted),
              ),
            ],
          ),
        ),
      );
    }

    return NotificationListener<ScrollNotification>(
      onNotification: _onScroll,
      child: RefreshIndicator(
        color: isp.accent,
        onRefresh: () async {
          setState(() {
            _items.clear();
            _page = 1;
            _hasMore = true;
            _initialLoaded = false;
          });
          await _loadInitial();
        },
        child: ListView.builder(
          padding: const EdgeInsets.only(bottom: 100),
          itemCount: _items.length + 1,
          itemBuilder: (context, index) {
            if (index == _items.length) {
              return _loadingMore
                  ? const Padding(
                      padding: EdgeInsets.all(24),
                      child: Center(
                        child: SizedBox(
                          width: 24,
                          height: 24,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                      ),
                    )
                  : const SizedBox.shrink();
            }
            return _InvoiceTile(inv: _items[index]);
          },
        ),
      ),
    );
  }
}

class _InvoiceTile extends StatelessWidget {
  const _InvoiceTile({required this.inv});
  final InvoiceModel inv;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final fmt = NumberFormat.simpleCurrency(name: inv.currencyCode);
    final dateFmt = DateFormat('d MMM yyyy', 'id_ID');
    final l10n = AppLocalizations.of(context);
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: () => GoRouter.of(context).push('/invoices/${inv.id}'),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
          decoration: BoxDecoration(
            border: Border(
              bottom: BorderSide(color: isp.border, width: 0.5),
            ),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    inv.invoiceNumber,
                    style: TextStyle(
                      fontSize: 14,
                      fontWeight: FontWeight.w600,
                      color: isp.textPrimary,
                    ),
                  ),
                  IspStatusBadge(
                    label: inv.statusLabel(),
                    tone: inv.isPaid
                        ? StatusTone.success
                        : inv.isOverdue
                            ? StatusTone.danger
                            : StatusTone.warning,
                  ),
                ],
              ),
              const SizedBox(height: 6),
              Text(
                '${l10n.dueOn ?? 'Jatuh tempo'} ${dateFmt.format(inv.dueDate)}',
                style: TextStyle(
                  fontSize: 12,
                  color: isp.textMuted,
                ),
              ),
              const SizedBox(height: 12),
              Text(
                fmt.format(inv.amount),
                style: TextStyle(
                  fontSize: 20,
                  fontWeight: FontWeight.w700,
                  color: isp.textPrimary,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
