import 'package:flutter/material.dart';

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

  Color _color() {
    switch (tone) {
      case StatusTone.success:
        return IspColors.success;
      case StatusTone.warning:
        return IspColors.warning;
      case StatusTone.danger:
        return IspColors.danger;
      case StatusTone.info:
        return IspColors.info;
      case StatusTone.primary:
        return IspColors.primary;
      case StatusTone.neutral:
        return IspColors.textTertiary;
    }
  }

  @override
  Widget build(BuildContext context) {
    final clamped = value.clamp(0.0, 1.0);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (label != null) ...[
          Text(
            label!,
            style: const TextStyle(
              fontSize: 11,
              color: IspColors.textTertiary,
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
            backgroundColor: IspColors.bgTertiary,
            valueColor: AlwaysStoppedAnimation<Color>(_color()),
          ),
        ),
      ],
    );
  }
}
