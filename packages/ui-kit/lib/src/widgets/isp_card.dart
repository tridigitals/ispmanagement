import 'package:flutter/material.dart';

import '../theme/isp_theme_colors.dart';
import '../theme/theme.dart';

/// Consistent card wrapper with padding, border radius, and optional shadow.
class IspCard extends StatelessWidget {
  const IspCard({
    super.key,
    this.child,
    this.padding,
    this.margin,
    this.showBorder = true,
    this.showShadow = false,
    this.onTap,
    this.borderRadius,
  });

  final Widget? child;
  final EdgeInsetsGeometry? padding;
  final EdgeInsetsGeometry? margin;
  final bool showBorder;
  final bool showShadow;
  final VoidCallback? onTap;
  final double? borderRadius;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final r = borderRadius ?? IspRadii.lg;
    final card = Container(
      margin: margin,
      decoration: BoxDecoration(
        color: isp.surface,
        borderRadius: BorderRadius.circular(r),
        border: showBorder ? Border.all(color: isp.borderSubtle) : null,
        boxShadow: showShadow ? IspShadows.sm : null,
      ),
      child: Padding(
        padding: padding ?? const EdgeInsets.all(IspSpacing.lg),
        child: child,
      ),
    );

    if (onTap != null) {
      return Material(
        color: Colors.transparent,
        borderRadius: BorderRadius.circular(r),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(r),
          child: card,
        ),
      );
    }
    return card;
  }
}
