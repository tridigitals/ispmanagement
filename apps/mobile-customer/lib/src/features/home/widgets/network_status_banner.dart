import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../../l10n/app_localizations.dart';
import '../../../services/network_status_providers.dart';

/// Sticky banner that shows outage/maintenance status on the home screen.
class NetworkStatusBanner extends ConsumerWidget {
  const NetworkStatusBanner({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
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
    final isp = context.isp;
    final (icon, color, text) = switch (status.status) {
      NetworkOperationalStatus.majorOutage => (
          Icons.signal_wifi_off,
          isp.danger,
          'Tidak ada koneksi di area ${status.area}',
        ),
      NetworkOperationalStatus.partialOutage => (
          Icons.warning_amber_rounded,
          isp.warning,
          'Gangguan sebagian di ${status.area}',
        ),
      NetworkOperationalStatus.degraded => (
          Icons.network_check,
          isp.warning,
          'Koneksi lambat di ${status.area}',
        ),
      NetworkOperationalStatus.maintenance => (
          Icons.engineering,
          isp.info,
          'Pemeliharaan jaringan di ${status.area}',
        ),
      _ => (Icons.info_outline, isp.info, status.statusLabel),
    };

    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 0),
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      color: color.withOpacity(0.12),
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
                    style: TextStyle(
                      fontSize: 12,
                      color: isp.textMuted,
                    ),
                  ),
                ],
                if (status.eta != null) ...[
                  const SizedBox(height: 2),
                  Text(
                    'Estimasi pulih: ${_fmtTime(status.eta!)}',
                    style: TextStyle(
                      fontSize: 11,
                      color: isp.textMuted,
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
