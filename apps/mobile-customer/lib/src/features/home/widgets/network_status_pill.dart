import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:ui_kit/ui_kit.dart';
import 'package:api_client/api_client.dart';
import 'package:mobile_customer/src/services/network_status_providers.dart';

/// Pill-shaped status chips — matches mockup `.status-pill` style.
class NetworkStatusPill extends ConsumerWidget {
  const NetworkStatusPill({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final async = ref.watch(networkStatusProvider);

    return async.maybeWhen(
      data: (status) {
        final isp = context.isp;

        return Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            // Online dot
            Container(
              width: 8,
              height: 8,
              margin: const EdgeInsets.only(right: 6),
              decoration: BoxDecoration(
                color: isp.success,
                shape: BoxShape.circle,
                boxShadow: [
                  BoxShadow(
                    color: isp.success.withOpacity(0.3),
                    blurRadius: 8,
                  ),
                ],
              ),
            ),
            Text(
              'Internet Aktif',
              style: TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.w600,
                color: isp.textPrimary,
              ),
            ),
          ],
        );
      },
      orElse: () {
        final isp = context.isp;
        return Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
          decoration: BoxDecoration(
            color: isp.surfaceTertiary,
            borderRadius: BorderRadius.circular(999),
            border: Border.all(color: isp.border, width: 1),
          ),
          child: Text(
            'Memeriksa...',
            style: TextStyle(fontSize: 12, color: isp.textMuted),
          ),
        );
      },
    );
  }
}
