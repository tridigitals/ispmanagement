import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../../services/feature_providers.dart';
import '../../../services/missing_providers.dart';

/// Swipeable announcement carousel.
/// Users can swipe left/right or let it auto-advance.
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

  // Pulse animation for icon
  late final AnimationController _pulseCtrl;
  late final Animation<double> _pulseAnim;

  @override
  void initState() {
    super.initState();
    _pulseCtrl = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 2),
    )..repeat(reverse: true);
    _pulseAnim = Tween<double>(begin: 0.75, end: 1.0).animate(
      CurvedAnimation(parent: _pulseCtrl, curve: Curves.easeInOut),
    );
  }

  @override
  void dispose() {
    _autoAdvance?.cancel();
    _pageCtrl.dispose();
    _pulseCtrl.dispose();
    super.dispose();
  }

  void _startAutoAdvance(int count) {
    _autoAdvance?.cancel();
    if (count <= 1) return;

    _autoAdvance = Timer.periodic(const Duration(seconds: 5), (_) {
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
                height: 80,
                child: PageView.builder(
                  controller: _pageCtrl,
                  itemCount: announcements.length,
                  onPageChanged: (i) => setState(() => _currentPage = i),
                  itemBuilder: (_, i) {
                    final a = announcements[i];
                    return _AnnouncementCard(
                      announcement: a,
                      pulseAnim: _pulseAnim,
                      bodyText: a.plainBody,
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
              // Dots indicator
              if (announcements.length > 1) ...[
                const SizedBox(height: 8),
                _DotsIndicator(
                  current: _currentPage,
                  total: announcements.length,
                ),
              ],
            ],
          ),
        );
      },
    );
  }
}

class _AnnouncementCard extends StatelessWidget {
  const _AnnouncementCard({
    required this.announcement,
    required this.pulseAnim,
    required this.bodyText,
    required this.onTap,
    required this.onDismiss,
  });

  final AnnouncementModel announcement;
  final Animation<double> pulseAnim;
  final String bodyText;
  final VoidCallback onTap;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: 16),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        decoration: BoxDecoration(
          gradient: LinearGradient(
            colors: [
              IspColors.warning.withOpacity(0.22),
              IspColors.warning.withOpacity(0.08),
            ],
          ),
          borderRadius: BorderRadius.circular(14),
          border: Border.all(
            color: IspColors.warning.withOpacity(0.45),
            width: 1.5,
          ),
          boxShadow: [
            BoxShadow(
              color: IspColors.warning.withOpacity(0.12),
              blurRadius: 10,
              offset: const Offset(0, 2),
            ),
          ],
        ),
        child: Row(
          children: [
            // Animated icon
            ScaleTransition(
              scale: pulseAnim,
              child: Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: IspColors.warning.withOpacity(0.20),
                  shape: BoxShape.circle,
                ),
                child: const Icon(
                  Icons.campaign_rounded,
                  size: 22,
                  color: IspColors.warning,
                ),
              ),
            ),
            const SizedBox(width: 12),
            // Title + body
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    announcement.title,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      fontSize: 15,
                      fontWeight: FontWeight.w700,
                      color: IspColors.warning,
                      height: 1.3,
                    ),
                  ),
                  if (bodyText.isNotEmpty) ...[
                    const SizedBox(height: 3),
                    Text(
                      bodyText,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12,
                        fontWeight: FontWeight.w400,
                        color: IspColors.warning.withOpacity(0.75),
                        height: 1.3,
                      ),
                    ),
                  ],
                ],
              ),
            ),
            const SizedBox(width: 8),
            // CTA chevron
            Icon(
              Icons.arrow_forward_ios_rounded,
              size: 14,
              color: IspColors.warning.withOpacity(0.55),
            ),
            const SizedBox(width: 6),
            // Dismiss
            GestureDetector(
              onTap: onDismiss,
              child: Padding(
                padding: const EdgeInsets.all(4),
                child: Icon(
                  Icons.close_rounded,
                  size: 16,
                  color: IspColors.warning.withOpacity(0.35),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _DotsIndicator extends StatelessWidget {
  const _DotsIndicator({required this.current, required this.total});
  final int current;
  final int total;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      mainAxisSize: MainAxisSize.min,
      children: List.generate(total, (i) {
        final active = i == current;
        return AnimatedContainer(
          duration: const Duration(milliseconds: 250),
          margin: const EdgeInsets.symmetric(horizontal: 2.5),
          width: active ? 12 : 5,
          height: 5,
          decoration: BoxDecoration(
            color: active
                ? IspColors.warning
                : IspColors.warning.withOpacity(0.20),
            borderRadius: BorderRadius.circular(3),
          ),
        );
      }),
    );
  }
}
