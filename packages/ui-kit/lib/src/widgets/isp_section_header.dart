import 'package:flutter/material.dart';

import '../theme/theme.dart';

/// Section header with a title on the left and an optional "See All" action.
class IspSectionHeader extends StatelessWidget {
  const IspSectionHeader({
    required this.title,
    super.key,
    this.actionLabel,
    this.onAction,
  });

  final String title;
  final String? actionLabel;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: IspSpacing.lg,
        vertical: IspSpacing.sm,
      ),
      child: Row(
        children: [
          Expanded(
            child: Text(
              title,
              style: const TextStyle(
                fontSize: 16,
                fontWeight: FontWeight.w600,
                color: IspColors.textPrimary,
              ),
            ),
          ),
          if (actionLabel != null && onAction != null)
            GestureDetector(
              onTap: onAction,
              child: Text(
                actionLabel!,
                style: const TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w500,
                  color: IspColors.primary,
                ),
              ),
            ),
        ],
      ),
    );
  }
}
