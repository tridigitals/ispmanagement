import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../l10n/app_localizations.dart';
import '../services/network_status_providers.dart';
import 'package:ui_kit/ui_kit.dart';

/// Sticky banner that shows outage/maintenance status on the home screen.
class NetworkStatusBanner extends ConsumerWidget {
  const NetworkStatusBanner({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context)!;
    final async = ref.watch(networkStatusProvider);

    return async.maybeWhen(
      data: (status) {
        if (status.isNormal) return const SizedBox.shrink();
        return _Banner(status: status);
      },
      orElse: () => const SizedBox.shrink(),
    );
  }
}

class _Banner extends StatelessWidget {
  const _Banner({required this.status});
  final NetworkStatusModel status;

  @override
  Widget build(BuildContext context) {
    final (icon, color, text) = switch (status.status) {
      NetworkOperationalStatus.majorOutage => (
        Icons.signal_wifi_off,
        IspColors.danger,
        'Tidak ada koneksi di area ${status.area}',
      ),
      NetworkOperationalStatus.partialOutage => (
        Icons.warning_amber_rounded,
        IspColors.warning,
        'Gangguan sebagian di ${status.area}',
      ),
      NetworkOperationalStatus.degraded => (
        Icons.network_check,
        IspColors.warning,
        'Koneksi lambat di ${status.area}',
      ),
      NetworkOperationalStatus.maintenance => (
        Icons.engineering,
        IspColors.info,
        'Pemeliharaan jaringan di ${status.area}',
      ),
      _ => (Icons.info_outline, IspColors.info, status.statusLabel),
    };

    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 0),
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      color: color.withValues(alpha: 0.12),
      child: Row(
        children: [
          Icon(icon, color: color, size: 20),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  text,
                  style: TextStyle(
                    color: color,
                    fontWeight: FontWeight.w600,
                    fontSize: 13,
                  ),
                ),
                if (status.message != null) ...[
                  const SizedBox(height: 2),
                  Text(
                    status.message!,
                    style: const TextStyle(
                      fontSize: 12,
                      color: IspColors.textTertiary,
                    ),
                  ),
                ],
                if (status.eta != null) ...[
                  const SizedBox(height: 2),
                  Text(
                    'Estimasi pulih: ${_fmtTime(status.eta!)}',
                    style: const TextStyle(
                      fontSize: 11,
                      color: IspColors.textTertiary,
                    ),
                  ),
                ],
              ],
            ),
          ),
          Icon(Icons.chevron_right, color: color, size: 18),
        ],
      ),
    );
  }

  String _fmtTime(DateTime dt) {
    final h = dt.hour.toString().padLeft(2, '0');
    final m = dt.minute.toString().padLeft(2, '0');
    return '${dt.day}/${dt.month} $h:$m';
  }
}
