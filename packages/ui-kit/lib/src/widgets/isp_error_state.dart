import 'package:flutter/material.dart';

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
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(IspSpacing.xl),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              padding: const EdgeInsets.all(IspSpacing.lg),
              decoration: BoxDecoration(
                color: IspColors.danger.withOpacity(0.1),
                shape: BoxShape.circle,
              ),
              child: Icon(icon, size: 40, color: IspColors.danger),
            ),
            const SizedBox(height: IspSpacing.lg),
            Text(
              title,
              style: const TextStyle(
                fontSize: 16,
                fontWeight: FontWeight.w600,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(
              message,
              style: const TextStyle(color: IspColors.textTertiary),
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
