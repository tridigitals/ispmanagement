import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';
import '../../theme/app_theme.dart';

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

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final l10n = AppLocalizations.of(context);

    if (!_initialLoaded) {
      return CustomScrollView(
        slivers: [
          SliverAppBar(
            title: Text(l10n.myTickets),
            pinned: true,
            actions: [
              IconButton(
                icon: const Icon(Icons.add),
                onPressed: () => GoRouter.of(context).push('/tickets/new'),
              ),
            ],
          ),
          SliverFillRemaining(
            hasScrollBody: false,
            child: Center(
                child: CircularProgressIndicator(color: isp.accent)),
          ),
        ],
      );
    }

    if (_initialError != null) {
      return CustomScrollView(
        slivers: [
          SliverAppBar(
            title: Text(l10n.myTickets),
            pinned: true,
            actions: [
              IconButton(
                icon: const Icon(Icons.add),
                onPressed: () => GoRouter.of(context).push('/tickets/new'),
              ),
            ],
          ),
          SliverFillRemaining(
            hasScrollBody: false,
            child: Center(
              child: Text(
                _initialError.toString(),
                style: TextStyle(color: isp.textSecondary),
              ),
            ),
          ),
        ],
      );
    }

    if (_items.isEmpty) {
      return CustomScrollView(
        slivers: [
          SliverAppBar(
            title: Text(l10n.myTickets),
            pinned: true,
            actions: [
              IconButton(
                icon: const Icon(Icons.add),
                onPressed: () => GoRouter.of(context).push('/tickets/new'),
              ),
            ],
          ),
          SliverFillRemaining(
            hasScrollBody: false,
            child: _EmptyState(
              icon: Icons.support_agent_outlined,
              title: l10n.noTickets,
              subtitle: l10n.createFirstTicket,
              actionLabel: l10n.newTicket,
              onAction: () => GoRouter.of(context).push('/tickets/new'),
            ),
          ),
        ],
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
        child: CustomScrollView(
          slivers: [
            SliverAppBar(
              title: Text(l10n.myTickets),
              pinned: true,
              actions: [
                IconButton(
                  icon: const Icon(Icons.add),
                  onPressed: () => GoRouter.of(context).push('/tickets/new'),
                ),
              ],
            ),
            // Category filter chips
            SliverToBoxAdapter(
              child: Padding(
                padding:
                    const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
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
              ),
            ),
            SliverPadding(
              padding: const EdgeInsets.only(bottom: 100),
              sliver: SliverList(
                delegate: SliverChildBuilderDelegate(
                  (context, index) {
                    if (index == _items.length) {
                      return _loadingMore
                          ? Padding(
                              padding: EdgeInsets.all(24),
                              child: Center(
                                child: SizedBox(
                                  width: 24,
                                  height: 24,
                                  child: CircularProgressIndicator(
                                    strokeWidth: 2,
                                    color: isp.accent,
                                  ),
                                ),
                              ),
                            )
                          : const SizedBox.shrink();
                    }
                    return _TicketTile(t: _items[index]);
                  },
                  childCount: _items.length + 1,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TicketTile extends StatelessWidget {
  const _TicketTile({required this.t});
  final TicketModel t;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final dateFmt = DateFormat('d MMM', 'id_ID');
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: () => GoRouter.of(context).push('/tickets/${t.id}'),
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
                children: [
                  Expanded(
                    child: Text(
                      t.subject,
                      style: TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w600,
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
                        vertical: 2,
                      ),
                      decoration: BoxDecoration(
                        color: isp.accent,
                        borderRadius: BorderRadius.circular(9999),
                      ),
                      child: Text(
                        '${t.unreadCount}',
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 11,
                        ),
                      ),
                    ),
                ],
              ),
              const SizedBox(height: 8),
              Row(
                children: [
                  IspStatusBadge(
                    label: t.statusLabel(),
                    tone: t.isOpen
                        ? StatusTone.info
                        : t.isClosed
                            ? StatusTone.neutral
                            : StatusTone.warning,
                  ),
                  if (t.category != null && t.category!.isNotEmpty) ...[
                    const SizedBox(width: 6),
                    IspStatusBadge(
                      label: t.categoryLabel(),
                      tone: StatusTone.neutral,
                    ),
                  ],
                  const SizedBox(width: 8),
                  Text(
                    '· ${dateFmt.format(t.updatedAt)}',
                    style: TextStyle(
                      fontSize: 12,
                      color: isp.textMuted,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({
    required this.icon,
    required this.title,
    this.subtitle,
    this.actionLabel,
    this.onAction,
  });

  final IconData icon;
  final String title;
  final String? subtitle;
  final String? actionLabel;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Padding(
      padding: const EdgeInsets.all(40),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            padding: const EdgeInsets.all(24),
            decoration: BoxDecoration(
              color: isp.surface,
              shape: BoxShape.circle,
              border: Border.all(color: isp.border),
            ),
            child: Icon(icon, size: 48, color: isp.textMuted),
          ),
          const SizedBox(height: 20),
          Text(
            title,
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w600,
              color: isp.textPrimary,
            ),
          ),
          if (subtitle != null) ...[
            const SizedBox(height: 4),
            Text(
              subtitle!,
              textAlign: TextAlign.center,
              style: TextStyle(
                fontSize: 13,
                color: isp.textMuted,
              ),
            ),
          ],
          if (actionLabel != null) ...[
            const SizedBox(height: 20),
            ElevatedButton(
              onPressed: onAction,
              child: Text(actionLabel!),
            ),
          ],
        ],
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


    final isp = context.isp;    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 7),
        decoration: BoxDecoration(
          color: selected ? isp.accent : isp.surface,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(
            color: selected ? isp.accent : isp.border,
          ),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w600,
            color: selected ? Colors.white : isp.textSecondary,
          ),
        ),
      ),
    );
  }
}
