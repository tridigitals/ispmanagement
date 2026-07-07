import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../services/service_providers.dart';
import '../../services/settings_providers.dart' show currentTabProvider;
import '../../utils/loading_skeleton.dart';

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
                  ? Padding(
                      padding: const EdgeInsets.all(24),
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
    final color = sub.isActive ? isp.success : isp.danger;
    // ponytail: compute days from endsAt, fallback to billingCycle label
    final days = sub.endsAt != null
        ? sub.endsAt!.difference(DateTime.now()).inDays
        : 0;
    final daysLabel = days > 0 ? '$days hari lagi' : 'Kadaluarsa';

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () => GoRouter.of(context).push('/subscriptions/${sub.id}'),
          borderRadius: BorderRadius.circular(20),
          child: Container(
            decoration: _nbCard(isp),
            clipBehavior: Clip.antiAlias,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Content row with left accent strip
                IntrinsicHeight(
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      // 4px color strip
                      Container(width: 4, color: color),
                      // Card content
                      Expanded(
                        child: Padding(
                          padding: const EdgeInsets.fromLTRB(14, 16, 14, 16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                mainAxisAlignment:
                                    MainAxisAlignment.spaceBetween,
                                children: [
                                  Expanded(
                                    child: Text(
                                      sub.packageName ?? 'Paket',
                                      style: TextStyle(
                                        fontSize: 14,
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
                                        : StatusTone.danger,
                                  ),
                                ],
                              ),
                              const SizedBox(height: 4),
                              Text(
                                sub.routerName ??
                                    sub.locationLabel ??
                                    '-',
                                style: TextStyle(
                                  fontSize: 11,
                                  color: isp.textMuted,
                                ),
                              ),
                              const SizedBox(height: 8),
                              Text(
                                fmt.format(sub.price),
                                style: TextStyle(
                                  fontSize: 18,
                                  fontWeight: FontWeight.w900,
                                  color: isp.textPrimary,
                                  letterSpacing: -1,
                                  height: 1.0,
                                ),
                              ),
                              const SizedBox(height: 8),
                              // Progress bar
                              ClipRRect(
                                borderRadius: BorderRadius.circular(1),
                                child: LinearProgressIndicator(
                                  value:
                                      sub.isActive ? 0.6 : 1.0,
                                  backgroundColor: isp.border,
                                  color: color,
                                  minHeight: 2,
                                ),
                              ),
                              const SizedBox(height: 4),
                              Text(
                                '$daysLabel lagi',
                                style: TextStyle(
                                  fontSize: 10,
                                  color: isp.textMuted,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                      // Chevron
                      Padding(
                        padding: const EdgeInsets.only(
                            right: 14, top: 16),
                        child: Icon(
                          Icons.chevron_right_rounded,
                          size: 20,
                          color: isp.textMuted,
                        ),
                      ),
                    ],
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
