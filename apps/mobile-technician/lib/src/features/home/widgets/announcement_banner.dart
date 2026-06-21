import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../../services/feature_providers.dart';
import '../../../services/missing_providers.dart';

/// Swipeable announcement carousel with severity-based styling.
class AnnouncementBanner extends ConsumerStatefulWidget {
  const AnnouncementBanner({super.key});

  @override
  ConsumerState<AnnouncementBanner> createState() =>
      _AnnouncementBannerState();
}

class _AnnouncementBannerState extends ConsumerState<AnnouncementBanner>
    with SingleTickerProviderStateMixin {
  final PageController _pageCtrl = PageController();
  Timer? _autoAdvance;
  bool _paused = false;
  int _currentPage = 0;

  @override
  void dispose() {
    _autoAdvance?.cancel();
    _pageCtrl.dispose();
    super.dispose();
  }

  void _startAutoAdvance(int count) {
    _autoAdvance?.cancel();
    if (count <= 1) return;
    _autoAdvance = Timer.periodic(const Duration(seconds: 6), (_) {
      if (_paused || !mounted) return;
      final next = (_currentPage + 1) % count;
      _pageCtrl.animateToPage(
        next,
        duration: const Duration(milliseconds: 400),
        curve: Curves.easeInOut,
      );
    });
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final async = ref.watch(activeAnnouncementsProvider);

    return async.when(
      loading: () => const SizedBox.shrink(),
      error: (_, __) => const SizedBox.shrink(),
      data: (announcements) {
        if (announcements.isEmpty) return const SizedBox.shrink();
        _startAutoAdvance(announcements.length);

        return GestureDetector(
          onPanDown: (_) => _paused = true,
          onPanEnd: (_) => _paused = false,
          onPanCancel: () => _paused = false,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              SizedBox(
                height: 130,
                child: PageView.builder(
                  controller: _pageCtrl,
                  itemCount: announcements.length,
                  onPageChanged: (i) => setState(() => _currentPage = i),
                  itemBuilder: (_, i) {
                    final a = announcements[i];
                    return _AnnouncementCard(
                      announcement: a,
                      onTap: () =>
                          GoRouter.of(context).push('/announcements/${a.id}'),
                      onDismiss: () async {
                        final svc = ref.read(announcementServiceProvider);
                        await svc.dismiss(a.id);
                        ref.invalidate(activeAnnouncementsProvider);
                      },
                    );
                  },
                ),
              ),
              if (announcements.length > 1) ...[
                const SizedBox(height: 10),
                _DotsIndicator(
                  current: _currentPage,
                  total: announcements.length,
                  severity: announcements[_currentPage].severity,
                ),
              ],
            ],
          ),
        );
      },
    );
  }
}

// ─── Severity colors ───────────────────────────────────────────

_SeverityTheme _themeFor(String severity, IspThemeColors isp) {
  switch (severity) {
    case 'error':
      return _SeverityTheme(
        primary: isp.danger,
        icon: Icons.error_outline_rounded,
        label: 'Penting',
      );
    case 'warning':
      return _SeverityTheme(
        primary: isp.warning,
        icon: Icons.warning_amber_rounded,
        label: 'Peringatan',
      );
    case 'success':
      return _SeverityTheme(
        primary: isp.success,
        icon: Icons.check_circle_outline_rounded,
        label: 'Berhasil',
      );
    default: // info
      return _SeverityTheme(
        primary: isp.info,
        icon: Icons.info_outline_rounded,
        label: 'Info',
      );
  }
}

class _SeverityTheme {
  const _SeverityTheme({
    required this.primary,
    required this.icon,
    required this.label,
  });
  final Color primary;
  final IconData icon;
  final String label;
}

// ─── Announcement Card ─────────────────────────────────────────

class _AnnouncementCard extends StatelessWidget {
  const _AnnouncementCard({
    required this.announcement,
    required this.onTap,
    required this.onDismiss,
  });

  final AnnouncementModel announcement;
  final VoidCallback onTap;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final theme = _themeFor(announcement.severity, isp);
    final bodyText = announcement.plainBody;

    return GestureDetector(
      onTap: onTap,
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: 16),
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: [
              theme.primary.withOpacity(0.15),
              theme.primary.withOpacity(0.05),
            ],
          ),
          borderRadius: BorderRadius.circular(16),
          border: Border.all(
            color: theme.primary.withOpacity(0.30),
            width: 1,
          ),
        ),
        child: Row(
          children: [
            // Left accent bar
            Container(
              width: 4,
              decoration: BoxDecoration(
                color: theme.primary,
                borderRadius: const BorderRadius.only(
                  topLeft: Radius.circular(16),
                  bottomLeft: Radius.circular(16),
                ),
              ),
            ),
            // Content
            Expanded(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(14, 14, 10, 14),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // Top row: severity badge + dismiss
                    Row(
                      children: [
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 8, vertical: 3),
                          decoration: BoxDecoration(
                            color: theme.primary.withOpacity(0.15),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(theme.icon, size: 12, color: theme.primary),
                              const SizedBox(width: 4),
                              Text(
                                theme.label,
                                style: TextStyle(
                                  fontSize: 10,
                                  fontWeight: FontWeight.w700,
                                  color: theme.primary,
                                  letterSpacing: 0.3,
                                ),
                              ),
                            ],
                          ),
                        ),
                        const Spacer(),
                        GestureDetector(
                          onTap: onDismiss,
                          child: Padding(
                            padding: const EdgeInsets.all(4),
                            child: Icon(
                              Icons.close_rounded,
                              size: 16,
                              color: isp.textMuted,
                            ),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 10),
                    // Title
                    Text(
                      announcement.title,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w700,
                        color: isp.textPrimary,
                        height: 1.3,
                      ),
                    ),
                    if (bodyText.isNotEmpty) ...[
                      const SizedBox(height: 4),
                      Text(
                        bodyText,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: 12,
                          color: isp.textSecondary,
                          height: 1.4,
                        ),
                      ),
                    ],
                    const SizedBox(height: 8),
                    // CTA row
                    Row(
                      children: [
                        Text(
                          'Baca selengkapnya',
                          style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.w700,
                            color: theme.primary,
                          ),
                        ),
                        const SizedBox(width: 4),
                        Icon(
                          Icons.arrow_forward_rounded,
                          size: 14,
                          color: theme.primary,
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ─── Dots Indicator ────────────────────────────────────────────

class _DotsIndicator extends StatelessWidget {
  const _DotsIndicator({
    required this.current,
    required this.total,
    required this.severity,
  });
  final int current;
  final int total;
  final String severity;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final color = _themeFor(severity, isp).primary;
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      mainAxisSize: MainAxisSize.min,
      children: List.generate(total, (i) {
        final active = i == current;
        return AnimatedContainer(
          duration: const Duration(milliseconds: 250),
          margin: const EdgeInsets.symmetric(horizontal: 2.5),
          width: active ? 16 : 5,
          height: 5,
          decoration: BoxDecoration(
            color: active ? color : color.withOpacity(0.20),
            borderRadius: BorderRadius.circular(3),
          ),
        );
      }),
    );
  }
}
