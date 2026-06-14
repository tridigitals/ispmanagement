import 'package:flutter/material.dart';

import '../theme/isp_theme_colors.dart';
import '../theme/theme.dart';
import 'isp_button.dart';

/// Empty state shown when a list or section has no data.
class IspEmptyState extends StatelessWidget {
  const IspEmptyState({
    required this.message,
    super.key,
    this.title,
    this.icon = Icons.inbox_outlined,
    this.actionLabel,
    this.onAction,
  });

  final String message;
  final String? title;
  final IconData icon;
  final String? actionLabel;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(IspSpacing.xl),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 48, color: isp.textMuted),
            const SizedBox(height: IspSpacing.md),
            if (title != null) ...[
              Text(
                title!,
                style: TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                  color: isp.textPrimary,
                ),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 6),
            ],
            Text(
              message,
              style: TextStyle(color: isp.textMuted),
              textAlign: TextAlign.center,
            ),
            if (actionLabel != null && onAction != null) ...[
              const SizedBox(height: IspSpacing.lg),
              IspPrimaryButton(label: actionLabel!, onPressed: onAction),
            ],
          ],
        ),
      ),
    );
  }
}
