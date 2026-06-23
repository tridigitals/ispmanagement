import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../services/notifications_providers.dart'
    show unreadNotificationsCountProvider;
import '../../services/service_providers.dart'
    show ticketServiceProvider, workOrderServiceProvider;
import '../../theme/app_theme.dart';
import '../../utils/loading_skeleton.dart';
import 'widgets/network_status_banner.dart';
import 'widgets/announcement_banner.dart';

// ─── Design tokens ──────────────────────────────────────────────

const _kCardRadius = 16.0;
const _kSectionSpacing = 16.0;

// ─── Home Tab (technician — Dashboard Ringkas) ──────────────────

class HomeTab extends ConsumerStatefulWidget {
  const HomeTab({super.key});

  @override
  ConsumerState<HomeTab> createState() => _HomeTabState();
}

class _HomeTabState extends ConsumerState<HomeTab> {
  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final user = ref.watch(currentUserProvider);
    final dashAsync = ref.watch(_dashboardProvider);
    final todayTasksAsync = ref.watch(_todayTasksProvider);

    final now = DateTime.now();
    final dateStr = DateFormat('EEEE, d MMMM yyyy', 'id').format(now);

    return RefreshIndicator(
      onRefresh: () async {
        ref.invalidate(_dashboardProvider);
        ref.invalidate(_todayTasksProvider);
        ref.invalidate(unreadNotificationsCountProvider);
        await Future.wait([
          ref.read(_dashboardProvider.future),
          ref.read(_todayTasksProvider.future),
        ]);
      },
      color: isp.accent,
      child: CustomScrollView(
        slivers: [
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(20, 12, 20, 100),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // ── Greeting header ──
                    _GreetingHeader(
                      userName: user?.name.split(' ').first ?? '',
                      dateStr: dateStr,
                    ),
                    const SizedBox(height: _kSectionSpacing),

                    // ── Combined quick stats (3 cards) ──
                    dashAsync.when(
                      loading: () => Row(
                        children: List.generate(3, (_) {
                          return Expanded(
                            child: Padding(
                              padding: const EdgeInsets.symmetric(horizontal: 4),
                              child: const IspSkeletonCard(height: 72),
                            ),
                          );
                        }),
                      ),
                      error: (e, _) => _ErrorCard(message: e.toString()),
                      data: (dash) => _QuickStatsRow(dash: dash),
                    ),

                    const SizedBox(height: _kSectionSpacing),

                    // ── Tasks Today hero ──
                    _TodayTasksSection(tasksAsync: todayTasksAsync),

                    const SizedBox(height: _kSectionSpacing),

                    // ── Network status (compact) ──
                    const NetworkStatusBanner(),

                    const SizedBox(height: _kSectionSpacing),

                    // ── Announcement (compact) ──
                    const AnnouncementBanner(),
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

// ─── Dashboard data model ───────────────────────────────────────

class _DashboardData {
  final int activeTickets;
  final int activeWorkOrders;
  final int todayWorkOrders;

  const _DashboardData({
    required this.activeTickets,
    required this.activeWorkOrders,
    required this.todayWorkOrders,
  });
}

// ─── Providers ──────────────────────────────────────────────────

final _dashboardProvider = FutureProvider<_DashboardData>((ref) async {
  final ticketSvc = ref.read(ticketServiceProvider);
  final woSvc = ref.read(workOrderServiceProvider);

  // Parallel fetch
  final results = await Future.wait([
    ticketSvc.stats(),
    woSvc.list(includeClosed: true, limit: 500),
  ]);

  final ticketStats = (results[0] as ServiceResult<TicketStats>).getOrThrow();
  final allWo = (results[1] as ServiceResult<List<WorkOrderModel>>).getOrThrow();

  final activeTickets = ticketStats.open + ticketStats.pending;
  int activeWo = 0;
  int todayWo = 0;
  final now = DateTime.now();
  final todayStart = DateTime(now.year, now.month, now.day);

  for (final wo in allWo) {
    if (wo.isActive) activeWo++;
    if (wo.scheduledAt != null &&
        wo.scheduledAt!.isAfter(todayStart) &&
        wo.scheduledAt!.isBefore(todayStart.add(const Duration(days: 1)))) {
      todayWo++;
    }
  }

  return _DashboardData(
    activeTickets: activeTickets,
    activeWorkOrders: activeWo,
    todayWorkOrders: todayWo,
  );
});

final _todayTasksProvider = FutureProvider<List<WorkOrderModel>>((ref) async {
  final svc = ref.read(workOrderServiceProvider);
  final result = await svc.list(limit: 200);
  final all = result.getOrThrow();
  final now = DateTime.now();
  final todayStart = DateTime(now.year, now.month, now.day);
  final todayEnd = todayStart.add(const Duration(days: 1));

  // Prioritize: scheduled today first, then assigned, then pending
  final today = <WorkOrderModel>[];
  final rest = <WorkOrderModel>[];

  for (final wo in all) {
    if (!wo.isActive) continue;
    if (wo.scheduledAt != null &&
        wo.scheduledAt!.isAfter(todayStart) &&
        wo.scheduledAt!.isBefore(todayEnd)) {
      today.add(wo);
    } else {
      rest.add(wo);
    }
  }
  // Sort by scheduled time
  today.sort((a, b) => (a.scheduledAt ?? a.createdAt)
      .compareTo(b.scheduledAt ?? b.createdAt));
  rest.sort((a, b) => (a.createdAt).compareTo(b.createdAt));

  return [...today, ...rest];
});

// ─── Greeting Header ────────────────────────────────────────────

class _GreetingHeader extends StatelessWidget {
  const _GreetingHeader({required this.userName, required this.dateStr});
  final String userName;
  final String dateStr;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Row(
      children: [
        // Avatar circle
        Container(
          width: 48,
          height: 48,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            gradient: LinearGradient(
              colors: [isp.accent, isp.accent.withOpacity(0.7)],
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
            ),
          ),
          child: Center(
            child: Text(
              userName.isNotEmpty ? userName[0].toUpperCase() : 'T',
              style: const TextStyle(
                color: Colors.white,
                fontSize: 20,
                fontWeight: FontWeight.w700,
              ),
            ),
          ),
        ),
        const SizedBox(width: 14),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                '${l10n.hiPrefix}, $userName 👋',
                style: TextStyle(
                  fontSize: 20,
                  fontWeight: FontWeight.w800,
                  color: isp.textPrimary,
                  height: 1.2,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                dateStr,
                style: TextStyle(
                  fontSize: 13,
                  color: isp.textMuted,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

// ─── Quick Stats Row (3 cards) ──────────────────────────────────

class _QuickStatsRow extends StatelessWidget {
  const _QuickStatsRow({required this.dash});
  final _DashboardData dash;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Row(
      children: [
        Expanded(
          child: GestureDetector(
            onTap: () => GoRouter.of(context).go('/?tab=1'),
            child: _QuickStatCard(
              icon: Icons.confirmation_number_outlined,
              count: dash.activeTickets,
              label: l10n.homeActiveTickets,
              color: isp.warning,
            ),
          ),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: GestureDetector(
            onTap: () => GoRouter.of(context).go('/?tab=2'),
            child: _QuickStatCard(
              icon: Icons.build_outlined,
              count: dash.activeWorkOrders,
              label: l10n.homeActiveTasks,
              color: isp.accent,
            ),
          ),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: GestureDetector(
            onTap: () => GoRouter.of(context).go('/?tab=2'),
            child: _QuickStatCard(
              icon: Icons.today_outlined,
              count: dash.todayWorkOrders,
              label: l10n.homeToday,
              color: isp.success,
            ),
          ),
        ),
      ],
    );
  }
}

class _QuickStatCard extends StatelessWidget {
  const _QuickStatCard({
    required this.icon,
    required this.count,
    required this.label,
    required this.color,
  });
  final IconData icon;
  final int count;
  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Container(
      padding: const EdgeInsets.symmetric(vertical: 14, horizontal: 10),
      decoration: BoxDecoration(
        color: isp.surface,
        borderRadius: BorderRadius.circular(_kCardRadius),
        border: Border.all(color: isp.border, width: 1),
      ),
      child: Column(
        children: [
          Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: color.withOpacity(0.12),
              borderRadius: BorderRadius.circular(10),
            ),
            child: Icon(icon, size: 20, color: color),
          ),
          const SizedBox(height: 8),
          Text(
            count.toString(),
            style: TextStyle(
              fontSize: 22,
              fontWeight: FontWeight.w800,
              color: isp.textPrimary,
              height: 1.0,
            ),
          ),
          const SizedBox(height: 2),
          Text(
            label,
            style: TextStyle(
              fontSize: 11,
              fontWeight: FontWeight.w500,
              color: isp.textMuted,
            ),
          ),
        ],
      ),
    );
  }
}

// ─── Tasks Today Section ────────────────────────────────────────

class _TodayTasksSection extends StatelessWidget {
  const _TodayTasksSection({required this.tasksAsync});
  final AsyncValue<List<WorkOrderModel>> tasksAsync;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Icon(Icons.flash_on_rounded, size: 20, color: isp.accent),
            const SizedBox(width: 6),
            Text(
              l10n.homeTasksToday,
              style: TextStyle(
                fontSize: 17,
                fontWeight: FontWeight.w700,
                color: isp.textPrimary,
              ),
            ),
            const Spacer(),
            TextButton(
              onPressed: () => GoRouter.of(context).go('/?tab=2'),
              child: Text(l10n.seeAll),
            ),
          ],
        ),
        const SizedBox(height: 10),
        tasksAsync.when(
          loading: () => const IspSkeletonList(itemCount: 2),
          error: (e, _) => _ErrorCard(message: e.toString()),
          data: (tasks) {
            if (tasks.isEmpty) {
              return Container(
                width: double.infinity,
                padding: const EdgeInsets.all(32),
                decoration: BoxDecoration(
                  color: isp.surface,
                  borderRadius: BorderRadius.circular(_kCardRadius),
                  border: Border.all(color: isp.border),
                ),
                child: Column(
                  children: [
                    Icon(Icons.beach_access_outlined,
                        size: 40, color: isp.textMuted),
                    const SizedBox(height: 8),
                    Text(
                      l10n.homeNoTasksToday,
                      style: TextStyle(color: isp.textMuted, fontSize: 14),
                    ),
                  ],
                ),
              );
            }

            final display = tasks.take(3).toList();
            return Column(
              children: display.map((wo) {
                return Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: _TaskHeroCard(wo: wo),
                );
              }).toList(),
            );
          },
        ),
      ],
    );
  }
}

class _TaskHeroCard extends StatelessWidget {
  const _TaskHeroCard({required this.wo});
  final WorkOrderModel wo;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: () => GoRouter.of(context).push('/work-orders/${wo.id}'),
        borderRadius: BorderRadius.circular(_kCardRadius),
        child: Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: isp.surface,
            borderRadius: BorderRadius.circular(_kCardRadius),
            border: Border.all(color: isp.border),
          ),
          child: Row(
            children: [
              // Status indicator
              Container(
                width: 4,
                height: 48,
                decoration: BoxDecoration(
                  color: _statusColor(wo.status),
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      wo.customerName ??
                          'Pelanggan #${wo.customerId.substring(0, 8)}',
                      style: TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w600,
                        color: isp.textPrimary,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Row(
                      children: [
                        if (wo.packageName != null) ...[
                          Icon(Icons.inventory_2_outlined,
                              size: 13, color: isp.textMuted),
                          const SizedBox(width: 3),
                          Text(
                            wo.packageName!,
                            style: TextStyle(
                                fontSize: 12, color: isp.textSecondary),
                          ),
                          const SizedBox(width: 12),
                        ],
                        Icon(Icons.location_on_outlined,
                            size: 13, color: isp.textMuted),
                        const SizedBox(width: 3),
                        Expanded(
                          child: Text(
                            wo.locationLabel ?? '-',
                            style: TextStyle(
                                fontSize: 12, color: isp.textSecondary),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 6),
                    Row(
                      children: [
                        _StatusChip(status: wo.status),
                        const SizedBox(width: 8),
                        if (wo.scheduledAt != null) ...[
                          Icon(Icons.schedule,
                              size: 12, color: isp.accent),
                          const SizedBox(width: 3),
                          Text(
                            DateFormat('HH:mm').format(wo.scheduledAt!),
                            style: TextStyle(
                              fontSize: 12,
                              fontWeight: FontWeight.w600,
                              color: isp.accent,
                            ),
                          ),
                        ],
                        const Spacer(),
                        Text(
                          l10n.homeTapToView,
                          style: TextStyle(
                              fontSize: 11, color: isp.accent),
                        ),
                        const SizedBox(width: 4),
                        Icon(Icons.arrow_forward_ios,
                            size: 12, color: isp.accent),
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
      default:
        return Colors.grey;
    }
  }
}

class _StatusChip extends StatelessWidget {
  const _StatusChip({required this.status});
  final String status;

  @override
  Widget build(BuildContext context) {
    final (label, color) = _chipInfo(status);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        label,
        style: TextStyle(
            fontSize: 10, fontWeight: FontWeight.w700, color: color),
      ),
    );
  }

  (String, Color) _chipInfo(String s) {
    switch (s) {
      case 'pending':
        return ('Pending', Colors.grey);
      case 'assigned':
        return ('Assigned', Colors.orange);
      case 'in_progress':
        return ('Proses', Colors.blue);
      case 'completed':
        return ('Selesai', Colors.green);
      default:
        return (s, Colors.grey);
    }
  }
}

// ─── Error card ─────────────────────────────────────────────────

class _ErrorCard extends StatelessWidget {
  const _ErrorCard({required this.message});
  final String message;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Container(
      margin: const EdgeInsets.symmetric(vertical: 8),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: isp.danger.withOpacity(0.08),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        children: [
          Icon(Icons.error_outline, size: 20, color: isp.danger),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              message,
              style: TextStyle(
                fontSize: 13,
                color: isp.danger,
                fontWeight: FontWeight.w500,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
