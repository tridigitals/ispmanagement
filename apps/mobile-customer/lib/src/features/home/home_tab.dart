import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../services/missing_providers.dart';
import '../../services/notifications_providers.dart' show unreadNotificationsCountProvider;
import '../../services/settings_providers.dart' show currentTabProvider;

import '../../theme/app_theme.dart';
import '../../utils/loading_skeleton.dart';
import 'widgets/network_status_banner.dart';
import 'widgets/announcement_banner.dart';

// ─── Design tokens (local) ──────────────────────────────────────

const _kCardRadius = 20.0;
const _kCardPadding = EdgeInsets.all(16);
const _kSectionSpacing = 20.0;
const _kElementSpacing = 12.0;

// ─── Home Tab ────────────────────────────────────────────────────

class HomeTab extends ConsumerStatefulWidget {
  const HomeTab({super.key});

  @override
  ConsumerState<HomeTab> createState() => _HomeTabState();
}

class _HomeTabState extends ConsumerState<HomeTab> {
  @override
  Widget build(BuildContext context) {
    // Reload data when this tab becomes active (IndexedStack keeps all tabs alive)
    ref.listen(currentTabProvider, (prev, next) {
      if (next == 0 && prev != next) {
        ref.invalidate(mySubscriptionsProvider);
        ref.invalidate(myInvoicesProvider);
        ref.invalidate(unreadNotificationsCountProvider);
      }
    });

    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final user = ref.watch(currentUserProvider);
    final subState = ref.watch(mySubscriptionsProvider);
    final invState = ref.watch(myInvoicesProvider);

    return RefreshIndicator(
      onRefresh: () async {
        ref.invalidate(mySubscriptionsProvider);
        ref.invalidate(myInvoicesProvider);
        ref.invalidate(unreadNotificationsCountProvider);
        await Future.wait([
          ref.read(mySubscriptionsProvider.future),
          ref.read(myInvoicesProvider.future),
        ]);
      },
      color: isp.accent,
      child: CustomScrollView(
        slivers: [
          // ── Body ──
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(20, 12, 20, 100),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // ── Hero subscription card ──
                    _PrimarySubscription(subState: subState),

                    const SizedBox(height: _kSectionSpacing),

                    // ── Network status banner ──
                    const NetworkStatusBanner(),

                    const SizedBox(height: _kSectionSpacing),

                    // ── Announcement banner ──
                    const AnnouncementBanner(),

                    // ── Recent invoices ──
                    _RecentInvoices(invState: invState),
                  ],
                ),
              ]),
            ),
          ),
        ],
      ),
    );
  }
}

// ─── Primary subscription hero ──────────────────────────────────

class _PrimarySubscription extends ConsumerStatefulWidget {
  const _PrimarySubscription({required this.subState});
  final AsyncValue<List<SubscriptionModel>> subState;

  @override
  ConsumerState<_PrimarySubscription> createState() =>
      _PrimarySubscriptionState();
}

class _PrimarySubscriptionState extends ConsumerState<_PrimarySubscription> {
  final PageController _pageCtrl = PageController(viewportFraction: 0.92);
  int _currentPage = 0;

  @override
  void dispose() {
    _pageCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final l10n = AppLocalizations.of(context);
    return widget.subState.when(
      loading: () => const IspSkeletonCard(height: 220),
      error: (e, _) => _ErrorCard(message: e.toString()),
      data: (page) {
        if (page.isEmpty) {
          return _EmptyState(label: l10n.noSubscription);
        }
        // Sort: active first
        final sorted = [
          ...page
        ]..sort((a, b) => a.isActive == b.isActive ? 0 : (a.isActive ? -1 : 1));

        return Column(
          children: [
            SizedBox(
              height: 240,
              child: PageView.builder(
                controller: _pageCtrl,
                itemCount: sorted.length,
                onPageChanged: (i) => setState(() => _currentPage = i),
                itemBuilder: (_, i) => Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 4),
                  child: _SubscriptionHeroCard(sub: sorted[i]),
                ),
              ),
            ),
            if (sorted.length > 1) ...[
              const SizedBox(height: 12),
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: List.generate(
                  sorted.length,
                  (i) => AnimatedContainer(
                    duration: const Duration(milliseconds: 250),
                    margin: const EdgeInsets.symmetric(horizontal: 3),
                    width: i == _currentPage ? 20 : 6,
                    height: 6,
                    decoration: BoxDecoration(
                      color: i == _currentPage
                          ? isp.accent
                          : isp.border,
                      borderRadius: BorderRadius.circular(3),
                    ),
                  ),
                ),
              ),
            ],
          ],
        );
      },
    );
  }
}

class _SubscriptionHeroCard extends StatelessWidget {
  const _SubscriptionHeroCard({required this.sub});
  final SubscriptionModel sub;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final l10n = AppLocalizations.of(context);
    final fmt = NumberFormat.simpleCurrency(name: sub.currencyCode);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: () => GoRouter.of(context).push('/subscriptions/${sub.id}'),
        borderRadius: BorderRadius.circular(_kCardRadius),
        child: Ink(
          decoration: BoxDecoration(
            color: isp.surface,
            borderRadius: BorderRadius.circular(_kCardRadius),
            border: Border.all(color: isp.border, width: 1),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Accent line at top
              Container(
                height: 3,
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.vertical(
                    top: Radius.circular(_kCardRadius),
                  ),
                  gradient: LinearGradient(
                    colors: [isp.accent, isp.accentLight],
                  ),
                ),
              ),

              Padding(
                padding: const EdgeInsets.fromLTRB(20, 20, 20, 20),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // Top row: package name + status
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Flexible(
                          child: Text(
                            (sub.packageName ?? l10n.internetPackage)
                                .toUpperCase(),
                            style: TextStyle(
                              color: isp.textSecondary,
                              fontSize: 12,
                              fontWeight: FontWeight.w500,
                              letterSpacing: 1,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        const SizedBox(width: 8),
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
                    const SizedBox(height: 24),

                    // Price — dominant element
                    Text(
                      fmt.format(sub.price),
                      style: TextStyle(
                        color: isp.textPrimary,
                        fontSize: 44,
                        fontWeight: FontWeight.w800,
                        letterSpacing: -1.5,
                        height: 1.0,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '/ ${sub.billingCycle}',
                      style: TextStyle(
                        color: isp.textMuted,
                        fontSize: 14,
                        fontWeight: FontWeight.w400,
                      ),
                    ),
                    const SizedBox(height: 20),

                    // Router / location info + chevron
                    Row(
                      children: [
                        Container(
                          padding: const EdgeInsets.all(8),
                          decoration: BoxDecoration(
                            color: isp.accent.withOpacity(0.12),
                            borderRadius: BorderRadius.circular(10),
                          ),
                          child: Icon(
                            Icons.router,
                            color: isp.accent,
                            size: 16,
                          ),
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Text(
                            sub.routerName ?? sub.locationLabel ?? '-',
                            style: TextStyle(
                              color: isp.textSecondary,
                              fontSize: 14,
                              fontWeight: FontWeight.w400,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        Icon(
                          Icons.chevron_right_rounded,
                          color: isp.textMuted,
                          size: 22,
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ─── Recent invoices ────────────────────────────────────────────

class _RecentInvoices extends StatelessWidget {
  const _RecentInvoices({required this.invState});
  final AsyncValue<List<InvoiceModel>> invState;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final l10n = AppLocalizations.of(context);
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                l10n.recentInvoices,
                style: TextStyle(
                  fontSize: 18,
                  fontWeight: FontWeight.w700,
                  color: isp.textPrimary,
                ),
              ),
              TextButton(
                onPressed: () => GoRouter.of(context).go('/?tab=2'),
                child: Text(l10n.seeAll),
              ),
            ],
          ),
        ),
        const SizedBox(height: _kElementSpacing),
        Container(
          decoration: BoxDecoration(
            color: isp.surface,
            borderRadius: BorderRadius.circular(_kCardRadius),
            border: Border.all(color: isp.border, width: 1),
          ),
          child: invState.when(
            loading: () => const IspSkeletonList(itemCount: 3),
            error: (e, _) => _ErrorCard(message: e.toString()),
            data: (page) {
              if (page.isEmpty) {
                return Padding(
                  padding: EdgeInsets.all(24),
                  child: Center(
                    child: Text(
                      'No invoices',
                      style: TextStyle(color: isp.textMuted),
                    ),
                  ),
                );
              }
              return Column(
                children: page.take(5).map((inv) {
                  final statusColor = inv.isPaid
                      ? isp.success
                      : inv.isOverdue
                          ? isp.danger
                          : isp.warning;
                  return Material(
                    color: Colors.transparent,
                    child: InkWell(
                      onTap: () =>
                          GoRouter.of(context).push('/invoices/${inv.id}'),
                      child: Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 16,
                          vertical: 14,
                        ),
                        decoration: BoxDecoration(
                          border: Border(
                            bottom: BorderSide(
                              color: isp.border,
                              width: 0.5,
                            ),
                          ),
                        ),
                        child: Row(
                          children: [
                            // Receipt icon
                            Container(
                              padding: const EdgeInsets.all(8),
                              decoration: BoxDecoration(
                                color: isp.surfaceElevated,
                                borderRadius: BorderRadius.circular(10),
                              ),
                              child: Icon(
                                Icons.receipt_outlined,
                                size: 18,
                                color: isp.textSecondary,
                              ),
                            ),
                            const SizedBox(width: 12),
                            // Invoice info
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    inv.invoiceNumber,
                                    style: TextStyle(
                                      fontSize: 14,
                                      fontWeight: FontWeight.w600,
                                      color: isp.textPrimary,
                                    ),
                                  ),
                                  if (inv.subscriptionLabel != null ||
                                      inv.notes != null) ...[
                                    const SizedBox(height: 2),
                                    Text(
                                      inv.subscriptionLabel ?? inv.notes ?? '',
                                      style: TextStyle(
                                        fontSize: 12,
                                        color: isp.textMuted,
                                      ),
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ],
                                ],
                              ),
                            ),
                            // Amount + status
                            Column(
                              crossAxisAlignment: CrossAxisAlignment.end,
                              children: [
                                Text(
                                  fmt.format(inv.amount),
                                  style: TextStyle(
                                    fontWeight: FontWeight.w600,
                                    fontSize: 14,
                                    color: isp.textPrimary,
                                  ),
                                ),
                                const SizedBox(height: 4),
                                Container(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 8,
                                    vertical: 3,
                                  ),
                                  decoration: BoxDecoration(
                                    color: statusColor.withOpacity(0.15),
                                    borderRadius: BorderRadius.circular(9999),
                                  ),
                                  child: Text(
                                    inv.statusLabel(),
                                    style: TextStyle(
                                      fontSize: 11,
                                      fontWeight: FontWeight.w600,
                                      color: statusColor,
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                  );
                }).toList(),
              );
            },
          ),
        ),
      ],
    );
  }
}

// ─── Error card ─────────────────────────────────────────────────

class _ErrorCard extends StatelessWidget {
  const _ErrorCard({required this.message});
  final String message;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Container(
      margin: const EdgeInsets.symmetric(vertical: 8),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: isp.danger.withOpacity(0.1),
        borderRadius: BorderRadius.circular(_kCardRadius),
        border: Border.all(
          color: isp.danger.withOpacity(0.25),
          width: 1,
        ),
      ),
      child: Row(
        children: [
          Icon(Icons.error_outline, color: isp.danger),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              message,
              style: TextStyle(
                fontSize: 13,
                color: isp.textPrimary,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// ─── Empty state ────────────────────────────────────────────────

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Container(
      padding: const EdgeInsets.all(32),
      decoration: BoxDecoration(
        color: isp.surface,
        borderRadius: BorderRadius.circular(_kCardRadius),
        border: Border.all(color: isp.border, width: 1),
      ),
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.inbox_outlined,
              size: 48,
              color: isp.textMuted,
            ),
            const SizedBox(height: 12),
            Text(
              label,
              style: TextStyle(color: isp.textMuted),
            ),
          ],
        ),
      ),
    );
  }
}
