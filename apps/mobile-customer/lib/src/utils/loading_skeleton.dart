import 'package:flutter/material.dart';
import 'package:shimmer/shimmer.dart';

import 'package:ui_kit/ui_kit.dart';

/// Shimmering skeleton placeholder used while data is loading.
class IspSkeleton extends StatelessWidget {
  const IspSkeleton({
    super.key,
    this.width,
    this.height = 16,
    this.radius,
  });

  final double? width;
  final double height;
  final double? radius;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Shimmer.fromColors(
      baseColor: isp.surface,
      highlightColor: isp.surfaceTertiary,
      child: Container(
        width: width,
        height: height,
        decoration: BoxDecoration(
          color: isp.surface,
          borderRadius: BorderRadius.circular(radius ?? IspRadii.sm),
        ),
      ),
    );
  }
}

/// A list of skeleton rows for use inside Card / ListView.
class IspSkeletonList extends StatelessWidget {
  const IspSkeletonList({super.key, this.itemCount = 4});
  final int itemCount;
  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Column(
      children: List.generate(
        itemCount,
        (i) => Padding(
          padding: const EdgeInsets.symmetric(
            horizontal: IspSpacing.md,
            vertical: IspSpacing.sm,
          ),
          child: Row(
            children: [
              const IspSkeleton(width: 40, height: 40, radius: 8),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    IspSkeleton(width: 120, height: 14),
                    const SizedBox(height: 6),
                    IspSkeleton(width: 180, height: 12),
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

/// Card-style skeleton for hero blocks (subscription card, invoice card).
class IspSkeletonCard extends StatelessWidget {
  const IspSkeletonCard({super.key, this.height = 180});
  final double height;
  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Container(
      height: height,
      decoration: BoxDecoration(
        color: isp.surface,
        borderRadius: BorderRadius.circular(IspRadii.lg),
      ),
      child: Padding(
        padding: const EdgeInsets.all(IspSpacing.lg),
        child: Shimmer.fromColors(
          baseColor: isp.surfaceTertiary,
          highlightColor: IspColors.bgHover,
          child: const Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              IspSkeleton(width: 100, height: 12),
              SizedBox(height: 12),
              IspSkeleton(width: 200, height: 32),
              SizedBox(height: 12),
              IspSkeleton(width: 140, height: 12),
            ],
          ),
        ),
      ),
    );
  }
}
