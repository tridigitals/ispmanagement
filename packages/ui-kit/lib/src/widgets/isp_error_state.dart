import 'package:flutter/material.dart';

import '../theme/isp_theme_colors.dart';
import '../theme/theme.dart';
import 'isp_button.dart';

/// Error state shown when an API call or data load fails.
class IspErrorState extends StatelessWidget {
  const IspErrorState({
    required this.message,
    super.key,
    this.title = 'Terjadi kesalahan',
    this.icon = Icons.cloud_off_outlined,
    this.onRetry,
  });

  final String message;
  final String title;
  final IconData icon;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(IspSpacing.xl),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              padding: const EdgeInsets.all(IspSpacing.lg),
              decoration: BoxDecoration(
                color: isp.dangerSurface,
                shape: BoxShape.circle,
              ),
              child: Icon(icon, size: 40, color: isp.danger),
            ),
            const SizedBox(height: IspSpacing.lg),
            Text(
              title,
              style: TextStyle(
                fontSize: 16,
                fontWeight: FontWeight.w600,
                color: isp.textPrimary,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(
              message,
              style: TextStyle(color: isp.textMuted),
              textAlign: TextAlign.center,
            ),
            if (onRetry != null) ...[
              const SizedBox(height: IspSpacing.lg),
              IspPrimaryButton(
                label: 'Coba lagi',
                onPressed: onRetry,
                icon: Icons.refresh,
              ),
            ],
          ],
        ),
      ),
    );
  }
}
