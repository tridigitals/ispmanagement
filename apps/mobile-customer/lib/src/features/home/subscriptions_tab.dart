import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../services/service_providers.dart';
import '../../services/settings_providers.dart' show currentTabProvider;
import '../../utils/loading_skeleton.dart';

class SubscriptionsTab extends ConsumerStatefulWidget {
  const SubscriptionsTab({super.key});

  @override
  ConsumerState<SubscriptionsTab> createState() => _SubscriptionsTabState();
}

class _SubscriptionsTabState extends ConsumerState<SubscriptionsTab> {
  final List<SubscriptionModel> _items = [];
  int _page = 1;
  bool _hasMore = true;
  bool _loadingMore = false;
  bool _initialLoaded = false;
  Object? _initialError;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadInitial());
    ref.listen(currentTabProvider, (prev, next) {
      if (next == 1 && prev != next) _loadInitial();
    });
  }

  Future<void> _loadInitial() async {
    try {
      final svc = ref.read(subscriptionServiceProvider);
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
      final svc = ref.read(subscriptionServiceProvider);
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
    ref.listen(currentTabProvider, (prev, next) {
      if (next == 1 && prev != next) _refreshForTabActivation();
    });

    final isp = context.isp;

    if (!_initialLoaded) {
      return const IspSkeletonList(itemCount: 4);
    }

    if (_initialError != null) {
      return Scaffold(
        body: IspErrorState(
          message: _initialError.toString(),
          onRetry: () {
            setState(() {
              _initialLoaded = false;
              _initialError = null;
            });
            _loadInitial();
          },
        ),
      );
    }

    if (_items.isEmpty) {
      return Scaffold(
        body: IspEmptyState(
          icon: Icons.wifi_off_outlined,
          title: 'Belum ada langganan',
          message: 'Hubungi admin untuk berlangganan',
        ),
      );
    }

    return NotificationListener<ScrollNotification>(
      onNotification: _onScroll,
      child: RefreshIndicator(
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
                  ? Padding(
                      padding: const EdgeInsets.all(24),
                      child: Center(
                        child: SizedBox(
                          width: 24,
                          height: 24,
                          child: CircularProgressIndicator(
                            strokeWidth: 2,
                          ),
                        ),
                      ),
                    )
                  : const SizedBox.shrink();
            }
            return _SubscriptionTile(sub: _items[index]);
          },
        ),
      ),
    );
  }
}

class _SubscriptionTile extends StatelessWidget {
  const _SubscriptionTile({required this.sub});
  final SubscriptionModel sub;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final fmt = NumberFormat.simpleCurrency(name: sub.currencyCode);

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () => GoRouter.of(context).push('/subscriptions/${sub.id}'),
          borderRadius: BorderRadius.circular(20),
          child: Container(
            decoration: _nbCard(isp),
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Color-coded left accent strip hack — done via left border tint
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Expanded(
                      child: Text(
                        sub.packageName ?? 'Paket',
                        style: TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.w700,
                          color: isp.textPrimary,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    IspStatusBadge(
                      label: sub.statusLabel(),
                      tone: sub.isActive
                          ? StatusTone.success
                          : sub.needsAttention
                              ? StatusTone.danger
                              : StatusTone.warning,
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Icon(Icons.router, size: 15, color: isp.textMuted),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        sub.routerName ?? sub.locationLabel ?? '-',
                        style:
                            TextStyle(fontSize: 13, color: isp.textMuted),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    if (sub.locationLabel != null && sub.routerName != null) ...[
                      const SizedBox(width: 8),
                      Icon(Icons.location_on_outlined,
                          size: 14, color: isp.textMuted),
                      const SizedBox(width: 4),
                      Flexible(
                        child: Text(
                          sub.locationLabel!,
                          style: TextStyle(fontSize: 12, color: isp.textMuted),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ],
                ),
                const SizedBox(height: 12),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          fmt.format(sub.price),
                          style: TextStyle(
                            fontSize: 20,
                            fontWeight: FontWeight.w800,
                            color: isp.textPrimary,
                            letterSpacing: -0.5,
                          ),
                        ),
                        Text(
                          '/ ${sub.billingCycle}',
                          style:
                              TextStyle(fontSize: 12, color: isp.textMuted),
                        ),
                      ],
                    ),
                    Container(
                      padding: const EdgeInsets.all(8),
                      decoration: BoxDecoration(
                        color: isp.surface,
                        borderRadius: BorderRadius.circular(10),
                        border: Border.all(color: isp.border, width: 1.5),
                        boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
                      ),
                      child: Icon(
                        Icons.arrow_forward_ios,
                        size: 18,
                        color: isp.textMuted,
                      ),
                    ),
                  ],
                ),
                // Progress bar (ponyato: use actual billing cycle days)
                const SizedBox(height: 10),
                ClipRRect(
                  borderRadius: BorderRadius.circular(2),
                  child: LinearProgressIndicator(
                    value: sub.isActive ? 0.6 : 1.0,
                    backgroundColor: isp.border,
                    color: sub.isActive ? isp.accent : isp.danger,
                    minHeight: 3,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
