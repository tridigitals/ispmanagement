import 'package:flutter/material.dart';

import '../theme/theme.dart';

class IspStatusBadge extends StatelessWidget {
  const IspStatusBadge({
    required this.label,
    this.tone = StatusTone.neutral,
    this.icon,
    super.key,
  });

  final String label;
  final StatusTone tone;
  final IconData? icon;

  Color _bg() {
    switch (tone) {
      case StatusTone.success:
        return IspColors.success.withOpacity(0.15);
      case StatusTone.warning:
        return IspColors.warning.withOpacity(0.15);
      case StatusTone.danger:
        return IspColors.danger.withOpacity(0.15);
      case StatusTone.info:
        return IspColors.info.withOpacity(0.15);
      case StatusTone.primary:
        return IspColors.primarySubtle;
      case StatusTone.neutral:
        return IspColors.bgTertiary;
    }
  }

  Color _fg() {
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
        return IspColors.textSecondary;
    }
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: _bg(),
        borderRadius: BorderRadius.circular(IspRadii.pill),
        border: Border.all(color: _fg().withOpacity(0.2), width: 0.5),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 12, color: _fg()),
            const SizedBox(width: 4),
          ],
          Text(
            label,
            style: TextStyle(
              fontSize: 11,
              fontWeight: FontWeight.w600,
              color: _fg(),
            ),
          ),
        ],
      ),
    );
  }
}
