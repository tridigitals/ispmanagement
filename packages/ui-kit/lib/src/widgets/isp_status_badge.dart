import 'package:flutter/material.dart';

import '../theme/isp_theme_colors.dart';
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

  Color _bg(IspThemeColors isp) {
    switch (tone) {
      case StatusTone.success:
        return isp.successSurface;
      case StatusTone.warning:
        return isp.warningSurface;
      case StatusTone.danger:
        return isp.dangerSurface;
      case StatusTone.info:
        return isp.infoSurface;
      case StatusTone.primary:
        return isp.accentSurface;
      case StatusTone.neutral:
        return isp.surfaceTertiary;
    }
  }

  Color _fg(IspThemeColors isp) {
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
        return isp.textSecondary;
    }
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final fg = _fg(isp);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: _bg(isp),
        borderRadius: BorderRadius.circular(IspRadii.pill),
        border: Border.all(color: fg.withOpacity(0.2), width: 0.5),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 12, color: fg),
            const SizedBox(width: 4),
          ],
          Text(
            label,
            style: TextStyle(
              fontSize: 11,
              fontWeight: FontWeight.w600,
              color: fg,
            ),
          ),
        ],
      ),
    );
  }
}
