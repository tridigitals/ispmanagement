import 'package:flutter/material.dart';

import '../theme/isp_theme_colors.dart';
import '../theme/theme.dart';
import 'isp_status_badge.dart';

class IspProgressBar extends StatelessWidget {
  const IspProgressBar({
    required this.value,
    this.label,
    this.tone = StatusTone.primary,
    this.height = 6,
    super.key,
  });

  final double value; // 0..1
  final String? label;
  final StatusTone tone;
  final double height;

  Color _color(IspThemeColors isp) {
    switch (tone) {
      case StatusTone.success:
        return isp.success;
      case StatusTone.warning:
        return isp.warning;
      case StatusTone.danger:
        return isp.danger;
      case StatusTone.info:
        return isp.info;
      case StatusTone.primary:
        return isp.accent;
      case StatusTone.neutral:
        return isp.textMuted;
    }
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final clamped = value.clamp(0.0, 1.0);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (label != null) ...[
          Text(
            label!,
            style: TextStyle(
              fontSize: 11,
              color: isp.textMuted,
              fontWeight: FontWeight.w500,
            ),
          ),
          const SizedBox(height: 4),
        ],
        ClipRRect(
          borderRadius: BorderRadius.circular(IspRadii.pill),
          child: LinearProgressIndicator(
            value: clamped,
            minHeight: height,
            backgroundColor: isp.surfaceTertiary,
            valueColor: AlwaysStoppedAnimation<Color>(_color(isp)),
          ),
        ),
      ],
    );
  }
}
