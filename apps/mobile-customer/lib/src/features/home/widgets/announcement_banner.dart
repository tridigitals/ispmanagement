import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';
import 'package:api_client/api_client.dart';

import '../../../services/feature_providers.dart';
import '../../../services/missing_providers.dart';

/// Swipeable announcement carousel with neubrutalist dark styling,
/// severity-colored left border strip, and swipe-to-dismiss.
class AnnouncementBanner extends ConsumerStatefulWidget {
  const AnnouncementBanner({super.key});

  @override
  ConsumerState<AnnouncementBanner> createState() => _AnnouncementBannerState();
}

class _AnnouncementBannerState extends ConsumerState<AnnouncementBanner>
    with SingleTickerProviderStateMixin {
  final PageController _pageCtrl = PageController(viewportFraction: 0.92);
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
                height: 146,
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

// ─── Severity colors ────────────────────────────────────────────

_SeverityTheme _themeFor(String severity, IspThemeColors isp) {
  switch (severity) {
    case 'error':
      return _SeverityTheme(
        primary: isp.danger,
        surface: isp.dangerSurface,
        icon: Icons.error_outline_rounded,
        label: 'Penting',
      );
    case 'warning':
      return _SeverityTheme(
        primary: isp.warning,
        surface: isp.warningSurface,
        icon: Icons.warning_amber_rounded,
        label: 'Peringatan',
      );
    case 'success':
      return _SeverityTheme(
        primary: isp.success,
        surface: isp.successSurface,
        icon: Icons.check_circle_outline_rounded,
        label: 'Info',
      );
    default: // info
      return _SeverityTheme(
        primary: isp.info,
        surface: isp.infoSurface,
        icon: Icons.campaign_outlined,
        label: 'Pengumuman',
      );
  }
}

class _SeverityTheme {
  const _SeverityTheme({
    required this.primary,
    required this.surface,
    required this.icon,
    required this.label,
  });
  final Color primary;
  final Color surface;
  final IconData icon;
  final String label;
}

// ─── Announcement Card (neubrutalist + severity sidebar) ────────

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

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4),
      child: GestureDetector(
        onTap: onTap,
        child: Container(
          decoration: BoxDecoration(
            color: isp.surface,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(
              color: theme.primary.withOpacity(0.4),
              width: 1.5,
            ),
            boxShadow: [
              BoxShadow(
                color: theme.primary.withOpacity(0.15),
                offset: const Offset(3, 3),
                blurRadius: 0,
              ),
            ],
          ),
          clipBehavior: Clip.antiAlias,
          child: IntrinsicHeight(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // Severity-colored left border strip
                Container(
                  width: 5,
                  color: theme.primary,
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
                                color: theme.surface,
                                borderRadius: BorderRadius.circular(6),
                                border: Border.all(
                                  color: theme.primary.withOpacity(0.3),
                                  width: 1,
                                ),
                              ),
                              child: Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  Icon(theme.icon,
                                      size: 12, color: theme.primary),
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
                        const SizedBox(height: 10),
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
        ),
      ),
    );
  }
}

// ─── Dots Indicator ─────────────────────────────────────────────

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
