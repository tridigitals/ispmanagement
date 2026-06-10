import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';
import '../../theme/app_theme.dart';
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
    // Defer first load to after build
    WidgetsBinding.instance.addPostFrameCallback((_) => _loadInitial());
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
    final l10n = AppLocalizations.of(context);

    // Still loading initial
    if (!_initialLoaded) {
      return CustomScrollView(
        slivers: [
          SliverAppBar(title: Text(l10n.mySubscriptions), pinned: true),
          const SliverFillRemaining(
            hasScrollBody: false,
            child: IspSkeletonList(itemCount: 4),
          ),
        ],
      );
    }

    // Initial error
    if (_initialError != null) {
      return CustomScrollView(
        slivers: [
          SliverAppBar(title: Text(l10n.mySubscriptions), pinned: true),
          SliverFillRemaining(
            hasScrollBody: false,
            child: IspErrorState(
              message: _initialError.toString(),
              onRetry: () {
                setState(() {
                  _initialLoaded = false;
                  _initialError = null;
                });
                _loadInitial();
              },
            ),
          ),
        ],
      );
    }

    // Empty
    if (_items.isEmpty) {
      return CustomScrollView(
        slivers: [
          SliverAppBar(title: Text(l10n.mySubscriptions), pinned: true),
          SliverFillRemaining(
            hasScrollBody: false,
            child: IspEmptyState(
              icon: Icons.wifi_off_outlined,
              title: 'Belum ada langganan',
              message: 'Hubungi admin untuk berlangganan',
            ),
          ),
        ],
      );
    }

    return NotificationListener<ScrollNotification>(
      onNotification: _onScroll,
      child: RefreshIndicator(
        color: AppColors.accent,
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
            SliverAppBar(title: Text(l10n.mySubscriptions), pinned: true),
            SliverPadding(
              padding: const EdgeInsets.only(bottom: 100),
              sliver: SliverList(
                delegate: SliverChildBuilderDelegate(
                  (context, index) {
                    if (index == _items.length) {
                      // Load-more indicator
                      return _loadingMore
                          ? const Padding(
                              padding: EdgeInsets.all(24),
                              child: Center(
                                child: SizedBox(
                                  width: 24,
                                  height: 24,
                                  child: CircularProgressIndicator(
                                    strokeWidth: 2,
                                    color: AppColors.accent,
                                  ),
                                ),
                              ),
                            )
                          : const SizedBox.shrink();
                    }
                    return _SubscriptionTile(sub: _items[index]);
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

class _SubscriptionTile extends StatelessWidget {
  const _SubscriptionTile({required this.sub});
  final SubscriptionModel sub;

  @override
  Widget build(BuildContext context) {
    final fmt = NumberFormat.simpleCurrency(name: sub.currencyCode);
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: () => GoRouter.of(context).push('/subscriptions/${sub.id}'),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
          decoration: const BoxDecoration(
            border: Border(
              bottom: BorderSide(color: AppColors.border, width: 0.5),
            ),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // ── Header row ──
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Expanded(
                    child: Text(
                      sub.packageName ?? 'Paket',
                      style: const TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.w600,
                        color: AppColors.textPrimary,
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
              // ── Router + Location ──
              Row(
                children: [
                  const Icon(Icons.router,
                      size: 15, color: AppColors.textMuted),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(
                      sub.routerName ?? sub.locationLabel ?? '-',
                      style: const TextStyle(
                        fontSize: 13,
                        color: AppColors.textMuted,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  if (sub.locationLabel != null && sub.routerName != null) ...[
                    const SizedBox(width: 8),
                    const Icon(Icons.location_on_outlined,
                        size: 14, color: AppColors.textMuted),
                    const SizedBox(width: 4),
                    Flexible(
                      child: Text(
                        sub.locationLabel!,
                        style: const TextStyle(
                          fontSize: 12,
                          color: AppColors.textMuted,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                ],
              ),
              const SizedBox(height: 12),
              // ── Price + detail hint ──
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        fmt.format(sub.price),
                        style: const TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w700,
                          color: AppColors.textPrimary,
                        ),
                      ),
                      Text(
                        '/ ${sub.billingCycle}',
                        style: const TextStyle(
                          fontSize: 12,
                          color: AppColors.textMuted,
                        ),
                      ),
                    ],
                  ),
                  Container(
                    padding: const EdgeInsets.all(6),
                    decoration: BoxDecoration(
                      color: AppColors.accent.withOpacity(0.12),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: const Icon(
                      Icons.arrow_forward_ios,
                      size: 14,
                      color: AppColors.accent,
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
