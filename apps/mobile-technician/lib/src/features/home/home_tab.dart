import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../services/missing_providers.dart';
import '../../services/notifications_providers.dart' show unreadNotificationsCountProvider;
import '../../services/service_providers.dart' show ticketServiceProvider;
import '../../theme/app_theme.dart';
import '../../utils/loading_skeleton.dart';
import 'widgets/network_status_banner.dart';
import 'widgets/announcement_banner.dart';
import '../tickets/ticket_l10n.dart';

// ─── Design tokens (local) ──────────────────────────────────────

const _kCardRadius = 20.0;
const _kSectionSpacing = 20.0;
const _kElementSpacing = 12.0;

// ─── Home Tab (technician) ──────────────────────────────────────

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
    final statsAsync = ref.watch(_ticketStatsProvider);
    final recentAsync = ref.watch(_recentTicketsProvider);

    return RefreshIndicator(
      onRefresh: () async {
        ref.invalidate(_ticketStatsProvider);
        ref.invalidate(_recentTicketsProvider);
        ref.invalidate(unreadNotificationsCountProvider);
        await Future.wait([
          ref.read(_ticketStatsProvider.future),
          ref.read(_recentTicketsProvider.future),
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
                    // ── Ticket stats row ──
                    _TicketStatsRow(statsAsync: statsAsync),

                    const SizedBox(height: _kSectionSpacing),

                    // ── Network status banner ──
                    const NetworkStatusBanner(),

                    const SizedBox(height: _kSectionSpacing),

                    // ── Announcement banner ──
                    const AnnouncementBanner(),

                    const SizedBox(height: _kSectionSpacing),

                    // ── Recent tickets ──
                    _RecentTickets(recentAsync: recentAsync),
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

// ─── Providers ──────────────────────────────────────────────────

final _ticketStatsProvider = FutureProvider<TicketStats>((ref) async {
  final svc = ref.read(ticketServiceProvider);
  final result = await svc.stats();
  return result.getOrThrow();
});

final _recentTicketsProvider = FutureProvider<List<TicketModel>>((ref) async {
  final svc = ref.read(ticketServiceProvider);
  final result = await svc.list(page: 1, perPage: 5);
  return result.getOrThrow().data;
});

// ─── Ticket stats row ───────────────────────────────────────────

class _TicketStatsRow extends StatelessWidget {
  const _TicketStatsRow({required this.statsAsync});
  final AsyncValue<TicketStats> statsAsync;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return statsAsync.when(
      loading: () => Row(
        children: List.generate(4, (_) {
          return Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4),
              child: const IspSkeletonCard(height: 80),
            ),
          );
        }),
      ),
      error: (e, _) => _ErrorCard(message: e.toString()),
      data: (stats) {
        final items = [
          (l10n.ticketStatsAll, stats.all, Icons.inbox_outlined, isp.textPrimary),
          (l10n.ticketStatsOpen, stats.open, Icons.error_outline, isp.warning),
          (l10n.ticketStatsPending, stats.pending, Icons.hourglass_empty, isp.info),
          (l10n.ticketStatsClosed, stats.closed, Icons.check_circle_outline, isp.success),
        ];

        return Row(
          children: items.map((item) {
            final (label, count, icon, color) = item;
            return Expanded(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4),
                child: _StatCard(
                  icon: icon,
                  label: label,
                  count: count,
                  color: color,
                ),
              ),
            );
          }).toList(),
        );
      },
    );
  }
}

class _StatCard extends StatelessWidget {
  const _StatCard({
    required this.icon,
    required this.label,
    required this.count,
    required this.color,
  });

  final IconData icon;
  final String label;
  final int count;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Container(
      padding: const EdgeInsets.symmetric(vertical: 14, horizontal: 8),
      decoration: BoxDecoration(
        color: isp.surface,
        borderRadius: BorderRadius.circular(_kCardRadius),
        border: Border.all(color: isp.border, width: 1),
      ),
      child: Column(
        children: [
          Icon(icon, size: 22, color: color),
          const SizedBox(height: 6),
          Text(
            count.toString(),
            style: TextStyle(
              fontSize: 20,
              fontWeight: FontWeight.w800,
              color: isp.textPrimary,
              height: 1.0,
            ),
          ),
          const SizedBox(height: 2),
          Text(
            label,
            style: TextStyle(
              fontSize: 10,
              fontWeight: FontWeight.w500,
              color: isp.textMuted,
            ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ],
      ),
    );
  }
}

// ─── Recent tickets ─────────────────────────────────────────────

class _RecentTickets extends StatelessWidget {
  const _RecentTickets({required this.recentAsync});
  final AsyncValue<List<TicketModel>> recentAsync;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                l10n.recentTickets,
                style: TextStyle(
                  fontSize: 18,
                  fontWeight: FontWeight.w700,
                  color: isp.textPrimary,
                ),
              ),
              TextButton(
                onPressed: () => GoRouter.of(context).go('/?tab=1'),
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
          child: recentAsync.when(
            loading: () => const IspSkeletonList(itemCount: 3),
            error: (e, _) => _ErrorCard(message: e.toString()),
            data: (tickets) {
              if (tickets.isEmpty) {
                return Padding(
                  padding: const EdgeInsets.all(24),
                  child: Center(
                    child: Text(
                      l10n.noAssignedTickets,
                      style: TextStyle(color: isp.textMuted),
                    ),
                  ),
                );
              }
              return Column(
                children: tickets.map((ticket) {
                  return Material(
                    color: Colors.transparent,
                    child: InkWell(
                      onTap: () =>
                          GoRouter.of(context).push('/tickets/${ticket.id}'),
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
                            // Status dot
                            Container(
                              width: 10,
                              height: 10,
                              decoration: BoxDecoration(
                                color: ticket.statusColor(),
                                shape: BoxShape.circle,
                              ),
                            ),
                            const SizedBox(width: 12),
                            // Ticket info
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    ticket.subject,
                                    style: TextStyle(
                                      fontSize: 14,
                                      fontWeight: FontWeight.w600,
                                      color: isp.textPrimary,
                                    ),
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                  const SizedBox(height: 2),
                                  Text(
                                    '#${ticket.id.substring(0, 8)} · ${ticket.statusLabel()}',
                                    style: TextStyle(
                                      fontSize: 12,
                                      color: isp.textMuted,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                            // Priority indicator
                            _PriorityBadge(priority: ticket.priorityLabel()),
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

// ─── Priority badge ─────────────────────────────────────────────

class _PriorityBadge extends StatelessWidget {
  const _PriorityBadge({required this.priority});
  final String priority;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final (label, color) = _priorityInfo(priority);

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(9999),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: color,
        ),
      ),
    );
  }

  (String, Color) _priorityInfo(String p) {
    switch (p.toLowerCase()) {
      case 'critical':
      case 'urgent':
        return ('!', Colors.red);
      case 'high':
        return ('!!', Colors.orange);
      case 'normal':
      case 'medium':
        return ('!', Colors.blue);
      default:
        return ('-', Colors.grey);
    }
  }
}

// ─── Status color helper ────────────────────────────────────────

extension _TicketStatusDisplay on TicketModel {
  String statusLabel() {
    switch (status) {
      case TicketStatus.open:
        return 'Open';
      case TicketStatus.inProgress:
        return 'In Progress';
      case TicketStatus.waitingCustomer:
        return 'Waiting Customer';
      case TicketStatus.waitingStaff:
        return 'Waiting Staff';
      case TicketStatus.resolved:
        return 'Resolved';
      case TicketStatus.closed:
        return 'Closed';
      case TicketStatus.cancelled:
        return 'Cancelled';
    }
  }

  Color statusColor() {
    switch (status) {
      case TicketStatus.open:
        return Colors.orange;
      case TicketStatus.inProgress:
        return Colors.blue;
      case TicketStatus.waitingCustomer:
        return Colors.purple;
      case TicketStatus.waitingStaff:
        return Colors.teal;
      case TicketStatus.resolved:
        return Colors.green;
      case TicketStatus.closed:
      case TicketStatus.cancelled:
        return Colors.grey;
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
