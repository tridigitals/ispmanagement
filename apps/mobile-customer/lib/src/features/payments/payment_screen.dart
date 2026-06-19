import 'package:api_client/api_client.dart' hide Success, Failure;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/feature_providers.dart';
import '../../services/payment_providers.dart';
import '../../services/public_settings_providers.dart';

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

    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final settingsAsync = ref.watch(publicSettingsProvider);
    final channelsAsync = ref.watch(
      paymentChannelsProvider(widget.invoiceId),
    );

    return Scaffold(
      appBar: AppBar(title: Text(l10n.choosePaymentMethod)),
      body: settingsAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (_, __) => _buildPaymentList(context, isp, l10n, null, channelsAsync),
        data: (settings) => _buildPaymentList(context, isp, l10n, settings, channelsAsync),
      ),
    );
  }

  Widget _buildPaymentList(
    BuildContext context,
    IspThemeColors isp,
    AppLocalizations l10n,
    PublicSettingsModel? settings,
    AsyncValue<List<PaymentChannel>> channelsAsync,
  ) {
    final midtransEnabled = settings?.paymentMidtransEnabled ?? false;
    final duitkuEnabled = settings?.paymentDuitkuEnabled ?? false;
    final manualEnabled = settings?.paymentManualEnabled ?? false;
    final bankAccounts = settings?.activeBankAccounts ?? [];

    // Gateways: only show if enabled in tenant settings
    final showMidtrans = midtransEnabled;
    final showDuitku = duitkuEnabled;
    final showManual = manualEnabled && bankAccounts.isNotEmpty;

    if (!showMidtrans && !showDuitku && !showManual && (channelsAsync.valueOrNull?.isEmpty ?? true)) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.payments_outlined, size: 64, color: isp.textMuted),
              const SizedBox(height: 16),
              Text(
                'Tidak ada metode pembayaran tersedia',
                style: TextStyle(color: isp.textMuted),
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      );
    }

    return ListView(
      padding: const EdgeInsets.all(IspSpacing.lg),
      children: [
        Text(
          'Pilih metode pembayaran yang Anda inginkan',
          style: TextStyle(fontSize: 14, color: isp.textMuted),
        ),
        const SizedBox(height: IspSpacing.lg),

        // Manual Bank Transfer
        if (showManual) ...[
          ...bankAccounts.map((bank) => Padding(
            padding: const EdgeInsets.only(bottom: IspSpacing.sm),
            child: _BankTransferTile(
              bank: bank,
              onTap: () => _showBankTransferDialog(context, isp, l10n, bank),
              onCopy: (text) => _copyToClipboard(context, text),
            ),
          )),
          const SizedBox(height: IspSpacing.md),
        ],

        // Midtrans
        if (showMidtrans)
          Padding(
            padding: const EdgeInsets.only(bottom: IspSpacing.md),
            child: _PaymentMethodTile(
              icon: Icons.payment,
              name: 'Midtrans',
              description: 'Virtual Account, QRIS, E-Wallet, Credit Card',
              onTap: () => _pay(context, ref, 'midtrans'),
            ),
          ),

        // Duitku
        if (showDuitku)
          Padding(
            padding: const EdgeInsets.only(bottom: IspSpacing.md),
            child: _PaymentMethodTile(
              icon: Icons.account_balance_wallet,
              name: 'Duitku',
              description: 'Virtual Account, Convenience Store, E-Wallet',
              onTap: () => _pay(context, ref, 'duitku'),
            ),
          ),

        // Dynamic channels from API
        channelsAsync.when(
          loading: () => const Padding(
            padding: EdgeInsets.only(top: IspSpacing.lg),
            child: Center(child: SizedBox(width: 24, height: 24, child: CircularProgressIndicator(strokeWidth: 2))),
          ),
          error: (_, __) => const SizedBox.shrink(),
          data: (channels) {
            if (channels.isEmpty) return const SizedBox.shrink();
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Divider(),
                const SizedBox(height: IspSpacing.md),
                Text('Metode Lainnya', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: isp.textSecondary)),
                const SizedBox(height: IspSpacing.md),
                ...channels.map((ch) => Padding(
                  padding: const EdgeInsets.only(bottom: IspSpacing.sm),
                  child: _PaymentChannelTile(channel: ch, onTap: () => _payChannel(context, ref, ch)),
                )),
              ],
            );
          },
        ),
      ],
    );
  }

  void _copyToClipboard(BuildContext context, String text) {
    Clipboard.setData(ClipboardData(text: text));
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Nomor rekening berhasil disalin'), duration: const Duration(seconds: 2)),
    );
  }

  void _showBankTransferDialog(
    BuildContext context,
    IspThemeColors isp,
    AppLocalizations l10n,
    BankAccountModel bank,
  ) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Transfer ke ${bank.bankName}'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _InfoRow(label: 'Bank', value: bank.bankName),
            const SizedBox(height: 8),
            _InfoRow(label: 'Atas Nama', value: bank.accountHolder),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(child: _InfoRow(label: 'No. Rekening', value: bank.accountNumber)),
                IconButton(
                  icon: const Icon(Icons.copy, size: 18),
                  onPressed: () {
                    _copyToClipboard(ctx, bank.accountNumber);
                  },
                ),
              ],
            ),
            const SizedBox(height: 16),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: isp.accentSurface,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Row(
                children: [
                  Icon(Icons.info_outline, size: 16, color: isp.accent),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'Lakukan pembayaran laluupload bukti transfer',
                      style: TextStyle(fontSize: 12, color: isp.accent),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () {
              Navigator.pop(ctx);
              _showUploadProofDialog(context, isp, l10n, bank);
            },
            child: const Text('Upload Bukti Bayar'),
          ),
        ],
      ),
    );
  }

  void _showUploadProofDialog(
    BuildContext context,
    IspThemeColors isp,
    AppLocalizations l10n,
    BankAccountModel bank,
  ) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Upload Bukti Transfer'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              'Fitur upload bukti transfer sedang dalam pengembangan. '
              'Saat ini silakan lakukan pembayaran dan tunggu konfirmasi manual dari admin.',
              style: TextStyle(fontSize: 13, color: isp.textSecondary),
            ),
          ],
        ),
        actions: [
          FilledButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('OK'),
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

/// Bank transfer tile — shows bank name, account number, masked.
class _BankTransferTile extends StatelessWidget {
  const _BankTransferTile({
    required this.bank,
    required this.onTap,
    required this.onCopy,
  });

  final BankAccountModel bank;
  final VoidCallback onTap;
  final void Function(String) onCopy;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return IspCard(
      onTap: onTap,
      child: Row(
        children: [
          Container(
            width: 48,
            height: 48,
            decoration: BoxDecoration(
              color: isp.accentSurface,
              borderRadius: BorderRadius.circular(IspRadii.md),
            ),
            child: Center(
              child: Text(
                bank.bankName.substring(0, bank.bankName.length.clamp(0, 3)).toUpperCase(),
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w700,
                  color: isp.accent,
                ),
              ),
            ),
          ),
          const SizedBox(width: IspSpacing.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Transfer ${bank.bankName}',
                  style: const TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  bank.maskedNumber,
                  style: TextStyle(
                    fontSize: 12,
                    color: isp.textMuted,
                    fontFamily: 'monospace',
                  ),
                ),
                Text(
                  bank.accountHolder,
                  style: TextStyle(
                    fontSize: 11,
                    color: isp.textMuted,
                  ),
                ),
              ],
            ),
          ),
          Icon(Icons.copy, size: 18, color: isp.textMuted),
          const SizedBox(width: 4),
          Icon(Icons.chevron_right, color: isp.textMuted),
        ],
      ),
    );
  }
}

/// Simple label-value row for info dialogs.
class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: TextStyle(fontSize: 11, color: isp.textMuted)),
        const SizedBox(height: 2),
        Text(value, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
      ],
    );
  }
}
