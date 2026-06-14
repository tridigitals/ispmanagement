import 'package:flutter/material.dart';

import '../theme/isp_theme_colors.dart';
import '../theme/theme.dart';
import 'isp_status_badge.dart';

class IspStatCard extends StatelessWidget {
  const IspStatCard({
    required this.label,
    required this.value,
    this.helper,
    this.icon,
    this.tone = StatusTone.primary,
    this.onTap,
    super.key,
  });

  final String label;
  final String value;
  final String? helper;
  final IconData? icon;
  final StatusTone tone;
  final VoidCallback? onTap;

  Color _accent(IspThemeColors isp) {
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
    final accent = _accent(isp);
    return Material(
      color: isp.surface,
      borderRadius: BorderRadius.circular(IspRadii.lg),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(IspRadii.lg),
        child: Container(
          padding: const EdgeInsets.all(IspSpacing.lg),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(IspRadii.lg),
            border: Border.all(color: isp.borderSubtle),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  if (icon != null) ...[
                    Container(
                      padding: const EdgeInsets.all(8),
                      decoration: BoxDecoration(
                        color: accent.withOpacity(0.12),
                        borderRadius: BorderRadius.circular(IspRadii.sm),
                      ),
                      child: Icon(icon, size: 16, color: accent),
                    ),
                    const SizedBox(width: 8),
                  ],
                  Expanded(
                    child: Text(
                      label,
                      style: TextStyle(
                        fontSize: 12,
                        color: isp.textMuted,
                        fontWeight: FontWeight.w500,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Text(
                value,
                style: TextStyle(
                  fontSize: 22,
                  fontWeight: FontWeight.w700,
                  color: isp.textPrimary,
                  height: 1.1,
                ),
              ),
              if (helper != null) ...[
                const SizedBox(height: 4),
                Text(
                  helper!,
                  style: TextStyle(
                    fontSize: 11,
                    color: isp.textMuted,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
