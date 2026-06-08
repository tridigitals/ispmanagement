import 'package:flutter/material.dart';

import '../theme/theme.dart';

/// Shimmer-style loading placeholder using a pulse animation.
///
/// Provides [box], [line], and [circle] factory constructors for common
/// placeholder shapes.
class IspShimmer extends StatefulWidget {
  const IspShimmer({
    super.key,
    this.width,
    this.height,
    this.borderRadius,
  });

  /// Rectangular placeholder (e.g. card skeleton).
  const IspShimmer.box({
    super.key,
    this.width = double.infinity,
    this.height = 120,
    this.borderRadius,
  });

  /// Single-line text placeholder.
  const IspShimmer.line({
    super.key,
    this.width = double.infinity,
    this.height = 14,
    this.borderRadius,
  });

  /// Circular placeholder (e.g. avatar).
  const IspShimmer.circle({
    super.key,
    this.width = 48,
    this.height = 48,
    this.borderRadius,
  }) : assert(width == height, 'Circle shimmer requires equal width & height');

  final double? width;
  final double? height;
  final double? borderRadius;

  @override
  State<IspShimmer> createState() => _IspShimmerState();
}

class _IspShimmerState extends State<IspShimmer>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _animation;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1400),
    )..repeat(reverse: true);
    _animation = Tween<double>(begin: 0.3, end: 0.7).animate(
      CurvedAnimation(parent: _controller, curve: Curves.easeInOut),
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final radius = widget.borderRadius ??
        (widget.width == widget.height ? (widget.width ?? 0) / 2 : IspRadii.md);

    return AnimatedBuilder(
      animation: _animation,
      builder: (context, child) {
        return Container(
          width: widget.width,
          height: widget.height,
          decoration: BoxDecoration(
            color: IspColors.bgTertiary.withOpacity(_animation.value),
            borderRadius: BorderRadius.circular(radius),
          ),
        );
      },
    );
  }
}
