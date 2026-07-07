import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';
import '../../services/settings_providers.dart' show currentTabProvider;
import '../tickets/ticket_l10n.dart';

// ─── Neubrutalist card ───────────────────────────────────────────

decoration: NbStyle.card(context), // neubrutalist
                // color: isp.surface,
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

class SupportTab extends ConsumerStatefulWidget {
  const SupportTab({super.key});

  @override
  ConsumerState<SupportTab> createState() => _SupportTabState();
}

class _SupportTabState extends ConsumerState<SupportTab> {
  final List<TicketModel> _items = [];
  int _page = 1;
  bool _hasMore = true;
  bool _loadingMore = false;
  bool _initialLoaded = false;
  Object? _initialError;
  String? _categoryFilter;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadInitial());
    ref.listen(currentTabProvider, (prev, next) {
      if (next == 3 && prev != next) _loadInitial();
    });
  }

  Future<void> _loadInitial() async {
    try {
      final svc = ref.read(ticketServiceProvider);
      final result =
          await svc.list(page: 1, perPage: 20, category: _categoryFilter);
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
      final svc = ref.read(ticketServiceProvider);
      final result = await svc.list(
          page: _page + 1, perPage: 20, category: _categoryFilter);
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

  bool _onScroll(Notification notification) {
    if (notification is ScrollNotification &&
        notification.metrics.extentAfter <
            notification.metrics.maxScrollExtent * 0.1) {
      _loadMore();
    }
    return false;
  }

  void _setCategoryFilter(String? cat) {
    if (cat == _categoryFilter) return;
    setState(() {
      _categoryFilter = cat;
      _items.clear();
      _page = 1;
      _hasMore = true;
      _initialLoaded = false;
    });
    _loadInitial();
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

  @override
  Widget build(BuildContext context) {
    ref.listen(currentTabProvider, (prev, next) {
      if (next == 3 && prev != next) _refreshForTabActivation();
    });

    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    if (!_initialLoaded) {
      return Center(
        child: CircularProgressIndicator(color: isp.accent),
      );
    }

    if (_initialError != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.error_outline, size: 48, color: isp.danger),
            const SizedBox(height: 12),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 24),
              child: Text(
                _initialError.toString(),
                style: TextStyle(color: isp.textSecondary),
              ),
            ),
            const SizedBox(height: 16),
            OutlinedButton.icon(
              style: NbStyle.accentButton(context, outline: true),
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
      );
    }

    return Column(
      children: [
        // Category filter chips
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            children: [
              _FilterChip(
                label: 'Semua',
                selected: _categoryFilter == null,
                onTap: () => _setCategoryFilter(null),
              ),
              const SizedBox(width: 8),
              _FilterChip(
                label: 'Umum',
                selected: _categoryFilter == 'general',
                onTap: () => _setCategoryFilter('general'),
              ),
              const SizedBox(width: 8),
              _FilterChip(
                label: 'Tagihan',
                selected: _categoryFilter == 'billing',
                onTap: () => _setCategoryFilter('billing'),
              ),
              const SizedBox(width: 8),
              _FilterChip(
                label: 'Teknis',
                selected: _categoryFilter == 'technical',
                onTap: () => _setCategoryFilter('technical'),
              ),
              const SizedBox(width: 8),
              _FilterChip(
                label: 'Instalasi',
                selected: _categoryFilter == 'installation',
                onTap: () => _setCategoryFilter('installation'),
              ),
            ],
          ),
        ),
        Expanded(
          child: _items.isEmpty
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.support_agent_outlined,
                          size: 64, color: isp.textMuted),
                      const SizedBox(height: 12),
                      Text(l10n.noTickets,
                          style: TextStyle(color: isp.textMuted)),
                      const SizedBox(height: 16),
                      ElevatedButton.icon(
                        style: NbStyle.accentButton(context),
                        onPressed: () =>
                            GoRouter.of(context).push('/tickets/new'),
                        icon: const Icon(Icons.add),
                        label: Text(l10n.newTicket),
                      ),
                    ],
                  ),
                )
              : NotificationListener<ScrollNotification>(
                  onNotification: _onScroll,
                  decoration: NbStyle.card(context), // neubrutalist
                                  // color: isp.surface,
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
                                      child: CircularProgressIndicator(
                                          strokeWidth: 2),
                                    ),
                                  ),
                                )
                              : const SizedBox.shrink();
                        }
                        return _TicketTile(t: _items[index]);
                      },
                    ),
                  ),
                ),
        ),
      ],
    );
  }
}

class _TicketTile extends StatelessWidget {
  const _TicketTile({required this.t});
  final TicketModel t;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final dateFmt = DateFormat('d MMM', 'id_ID');

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () => GoRouter.of(context).push('/tickets/${t.id}'),
          borderRadius: BorderRadius.circular(20),
          child: Container(
            decoration: _nbCard(isp),
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        t.subject,
                        style: TextStyle(
                          fontSize: 15,
                          fontWeight: FontWeight.w700,
                          color: isp.textPrimary,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    if (t.unreadCount > 0)
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 3,
                        ),
                        decoration: NbStyle.card(context), // neubrutalist
                                        // color: isp.surface,
                          borderRadius: BorderRadius.circular(999),
                        ),
                        child: Text(
                          '${t.unreadCount}',
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 11,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                      ),
                  ],
                ),
                const SizedBox(height: 10),
                Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children: [
                    IspStatusBadge(
                      label: l10n.ticketStatusLabel(t.status),
                      tone: t.isOpen
                          ? StatusTone.info
                          : t.isClosed
                              ? StatusTone.neutral
                              : StatusTone.warning,
                    ),
                    if (t.category != null && t.category!.isNotEmpty)
                      IspStatusBadge(
                        label: l10n.ticketCategoryLabel(t.category),
                        tone: StatusTone.neutral,
                      ),
                    Text(
                      '· ${dateFmt.format(t.updatedAt)}',
                      style: TextStyle(fontSize: 12, color: isp.textMuted),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _FilterChip extends StatelessWidget {
  const _FilterChip({
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        decoration: NbStyle.card(context), // neubrutalist
                        // color: isp.surface,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(
            color: selected ? isp.accent : isp.border,
            width: 1.5,
          ),
          boxShadow: selected
              ? [
                  BoxShadow(
                    color: isp.accent.withOpacity(0.2),
                    offset: const Offset(2, 2),
                    blurRadius: 0,
                  ),
                ]
              : null,
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w700,
            color: selected ? Colors.white : isp.textSecondary,
          ),
        ),
      ),
    );
  }
}
