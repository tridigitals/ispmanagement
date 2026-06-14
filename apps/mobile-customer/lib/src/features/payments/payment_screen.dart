import 'package:api_client/api_client.dart' hide Success, Failure;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/feature_providers.dart';
import '../../services/payment_providers.dart';

/// Payment method picker: Midtrans or Duitku, with dynamic channel listing.
class PaymentScreen extends ConsumerStatefulWidget {
  const PaymentScreen({required this.invoiceId, super.key});
  final String invoiceId;

  @override
  ConsumerState<PaymentScreen> createState() => _PaymentScreenState();
}

class _PaymentScreenState extends ConsumerState<PaymentScreen> {
  @override
  void initState() {
    super.initState();
    // Trigger loading of payment channels.
    // The provider auto-fetches; we just need to watch it.
  }

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final l10n = AppLocalizations.of(context);
    final channelsAsync = ref.watch(
      paymentChannelsProvider(widget.invoiceId),
    );

    return Scaffold(
      appBar: AppBar(title: Text(l10n.choosePaymentMethod)),
      body: ListView(
        padding: const EdgeInsets.all(IspSpacing.lg),
        children: [
          Text(
            'Pilih metode pembayaran yang Anda inginkan',
            style: TextStyle(
              fontSize: 14,
              color: isp.textMuted,
            ),
          ),
          const SizedBox(height: IspSpacing.lg),

          // ── Gateway quick-picks ──
          _PaymentMethodTile(
            icon: Icons.payment,
            name: 'Midtrans',
            description: 'Virtual Account, QRIS, E-Wallet, Credit Card',
            onTap: () => _pay(context, ref, 'midtrans'),
          ),
          const SizedBox(height: IspSpacing.md),
          _PaymentMethodTile(
            icon: Icons.account_balance_wallet,
            name: 'Duitku',
            description: 'Virtual Account, Convenience Store, E-Wallet',
            onTap: () => _pay(context, ref, 'duitku'),
          ),

          // ── Dynamic payment channels from API ──
          channelsAsync.when(
            loading: () => const Padding(
              padding: EdgeInsets.only(top: IspSpacing.lg),
              child: Center(
                child: SizedBox(
                  width: 24,
                  height: 24,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
            ),
            error: (e, _) => const SizedBox.shrink(), // Silently ignore errors
            data: (channels) {
              if (channels.isEmpty) return const SizedBox.shrink();
              return Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const SizedBox(height: IspSpacing.lg),
                  const Divider(),
                  const SizedBox(height: IspSpacing.md),
                  Text(
                    'Metode Lainnya',
                    style: TextStyle(
                      fontSize: 14,
                      fontWeight: FontWeight.w600,
                      color: isp.textSecondary,
                    ),
                  ),
                  const SizedBox(height: IspSpacing.md),
                  ...channels.map(
                    (ch) => Padding(
                      padding: const EdgeInsets.only(bottom: IspSpacing.sm),
                      child: _PaymentChannelTile(
                        channel: ch,
                        onTap: () => _payChannel(context, ref, ch),
                      ),
                    ),
                  ),
                ],
              );
            },
          ),
        ],
      ),
    );
  }

  Future<void> _pay(
    BuildContext context,
    WidgetRef ref,
    String gateway,
  ) async {
    final l10n = AppLocalizations.of(context);
    final svc = ref.read(paymentServiceProvider);
    final result = gateway == 'midtrans'
        ? await svc.initiateMidtrans(widget.invoiceId)
        : await svc.initiateDuitku(widget.invoiceId);

    result.fold(
      (url) async {
        if (url.isNotEmpty) {
          if (context.mounted) {
            context.push(
              '/payments/${widget.invoiceId}/webview',
              extra: url,
            );
          }
        } else if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                l10n.noPaymentUrl ?? 'Tidak ada URL pembayaran',
              ),
            ),
          );
        }
      },
      (error) {
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(error.message)),
          );
        }
      },
    );
  }

  Future<void> _payChannel(
    BuildContext context,
    WidgetRef ref,
    PaymentChannel channel,
  ) async {
    final l10n = AppLocalizations.of(context);
    final svc = ref.read(paymentServiceProvider);

    // Determine which gateway to use based on channel
    // Duitku for convenience store methods, Midtrans for others
    final isDuitku = channel.method == PaymentMethod.convenienceStore;
    final result = isDuitku
        ? await svc.initiateDuitku(widget.invoiceId)
        : await svc.initiateMidtrans(widget.invoiceId);

    result.fold(
      (url) async {
        if (url.isNotEmpty) {
          if (context.mounted) {
            context.push(
              '/payments/${widget.invoiceId}/webview',
              extra: url,
            );
          }
        } else if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                l10n.noPaymentUrl ?? 'Tidak ada URL pembayaran',
              ),
            ),
          );
        }
      },
      (error) {
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(error.message)),
          );
        }
      },
    );
  }
}

class _PaymentMethodTile extends StatelessWidget {
  const _PaymentMethodTile({
    required this.icon,
    required this.name,
    required this.description,
    required this.onTap,
  });

  final IconData icon;
  final String name;
  final String description;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return IspCard(
      onTap: onTap,
      child: Row(
        children: [
          Container(
            width: 56,
            height: 56,
            decoration: BoxDecoration(
              color: isp.accentSurface,
              borderRadius: BorderRadius.circular(IspRadii.md),
            ),
            child: Icon(icon, color: isp.accent, size: 28),
          ),
          const SizedBox(width: IspSpacing.lg),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  name,
                  style: const TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: IspSpacing.xs),
                Text(
                  description,
                  style: TextStyle(
                    fontSize: 12,
                    color: isp.textMuted,
                  ),
                ),
              ],
            ),
          ),
          Icon(Icons.chevron_right, color: isp.textMuted),
        ],
      ),
    );
  }
}

/// Dynamic payment channel tile from the API.
class _PaymentChannelTile extends StatelessWidget {
  const _PaymentChannelTile({
    required this.channel,
    required this.onTap,
  });

  final PaymentChannel channel;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return IspCard(
      onTap: onTap,
      child: Row(
        children: [
          Container(
            width: 44,
            height: 44,
            decoration: BoxDecoration(
              color: isp.surfaceTertiary,
              borderRadius: BorderRadius.circular(IspRadii.md),
            ),
            child: Icon(
              _iconForMethod(channel.method),
              color: isp.accent,
              size: 22,
            ),
          ),
          const SizedBox(width: IspSpacing.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  channel.name,
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: IspSpacing.xs),
                Text(
                  '${channel.methodLabel}${channel.fee > 0 ? ' • Biaya: Rp ${channel.fee.toStringAsFixed(0)}' : ''}',
                  style: TextStyle(
                    fontSize: 11,
                    color: isp.textMuted,
                  ),
                ),
              ],
            ),
          ),
          Icon(Icons.chevron_right, color: isp.textMuted),
        ],
      ),
    );
  }

  IconData _iconForMethod(PaymentMethod method) {
    switch (method) {
      case PaymentMethod.virtualAccount:
        return Icons.account_balance;
      case PaymentMethod.ewallet:
        return Icons.account_balance_wallet;
      case PaymentMethod.qris:
        return Icons.qr_code;
      case PaymentMethod.creditCard:
        return Icons.credit_card;
      case PaymentMethod.bankTransfer:
        return Icons.swap_horiz;
      case PaymentMethod.convenienceStore:
        return Icons.store;
      case PaymentMethod.unknown:
        return Icons.payment;
    }
  }
}
