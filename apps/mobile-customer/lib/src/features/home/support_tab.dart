import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';

class SupportTab extends ConsumerWidget {
  const SupportTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context)!;
    final state = ref.watch(myTicketsProvider);
    return CustomScrollView(
      slivers: [
        SliverAppBar(
          title: Text(l10n.myTickets),
          pinned: true,
          actions: [
            IconButton(
              icon: const Icon(Icons.add),
              onPressed: () => context.push('/tickets/new'),
            ),
          ],
        ),
        state.when(
          loading: () => const SliverFillRemaining(
            hasScrollBody: false,
            child: Center(child: CircularProgressIndicator()),
          ),
          error: (e, _) => SliverFillRemaining(
            hasScrollBody: false,
            child: Center(child: Text(e.toString())),
          ),
          data: (page) {
            if (page.data.isEmpty) {
              return SliverFillRemaining(
                hasScrollBody: false,
                child: EmptyState(
                  icon: Icons.support_agent_outlined,
                  title: l10n.noTickets,
                  subtitle: l10n.createFirstTicket,
                  actionLabel: l10n.newTicket,
                  onAction: () => context.push('/tickets/new'),
                ),
              );
            }
            return SliverList.separated(
              itemBuilder: (_, i) => _TicketTile(t: page.data[i]),
              separatorBuilder: (_, __) => const SizedBox(height: 8),
              itemCount: page.data.length,
            );
          },
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
    final dateFmt = DateFormat('d MMM', 'id_ID');
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: IspSpacing.lg),
      child: Card(
        child: InkWell(
          borderRadius: BorderRadius.circular(IspRadii.lg),
          onTap: () => context.push('/tickets/${t.id}'),
          child: Padding(
            padding: const EdgeInsets.all(IspSpacing.lg),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        t.subject,
                        style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    if (t.unreadCount > 0)
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                        decoration: BoxDecoration(
                          color: IspColors.primary,
                          borderRadius: BorderRadius.circular(IspRadii.pill),
                        ),
                        child: Text(
                          '${t.unreadCount}',
                          style: const TextStyle(color: Colors.white, fontSize: 11),
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
                    const SizedBox(width: 8),
                    Text(
                      '· ${dateFmt.format(t.updatedAt)}',
                      style: const TextStyle(fontSize: 12, color: IspColors.textTertiary),
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

class EmptyState extends StatelessWidget {
  const EmptyState({
    required this.icon,
    required this.title,
    this.subtitle,
    this.actionLabel,
    this.onAction,
    super.key,
  });

  final IconData icon;
  final String title;
  final String? subtitle;
  final String? actionLabel;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(IspSpacing.xxl),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            padding: const EdgeInsets.all(IspSpacing.xl),
            decoration: BoxDecoration(
              color: IspColors.bgSurface,
              shape: BoxShape.circle,
              border: Border.all(color: IspColors.borderSubtle),
            ),
            child: Icon(icon, size: 48, color: IspColors.textTertiary),
          ),
          const SizedBox(height: IspSpacing.lg),
          Text(
            title,
            style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
          ),
          if (subtitle != null) ...[
            const SizedBox(height: 4),
            Text(
              subtitle!,
              textAlign: TextAlign.center,
              style: const TextStyle(fontSize: 13, color: IspColors.textTertiary),
            ),
          ],
          if (actionLabel != null) ...[
            const SizedBox(height: IspSpacing.lg),
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
