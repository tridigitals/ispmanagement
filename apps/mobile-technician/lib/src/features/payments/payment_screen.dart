import 'package:api_client/api_client.dart' hide Success, Failure;
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/feature_providers.dart';
import '../../services/payment_providers.dart';
import '../../services/public_settings_providers.dart';
import '../../services/missing_providers.dart';

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
      barrierDismissible: false,
      builder: (ctx) => _UploadProofDialog(
        invoiceId: widget.invoiceId,
        bank: bank,
        isp: isp,
        l10n: l10n,
        onSuccess: () {
          Navigator.pop(ctx);
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Bukti pembayaran berhasil diunggah. Menunggu konfirmasi admin.'),
              backgroundColor: isp.success,
            ),
          );
          // Refresh invoice to show updated status
          ref.invalidate(invoiceByIdProvider(widget.invoiceId));
        },
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
    final isProd = ref.read(publicSettingsProvider).valueOrNull
            ?.paymentMidtransIsProduction ??
        false;
    final result = gateway == 'midtrans'
        ? await svc.initiateMidtrans(widget.invoiceId, isProduction: isProd)
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
    final isProd = ref.read(publicSettingsProvider).valueOrNull
            ?.paymentMidtransIsProduction ??
        false;

    // Determine which gateway to use based on channel
    // Duitku for convenience store methods, Midtrans for others
    final isDuitku = channel.method == PaymentMethod.convenienceStore;
    final result = isDuitku
        ? await svc.initiateDuitku(widget.invoiceId)
        : await svc.initiateMidtrans(widget.invoiceId, isProduction: isProd);

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

/// Stateful upload proof dialog — picks file and uploads via StorageService.
class _UploadProofDialog extends ConsumerStatefulWidget {
  const _UploadProofDialog({
    required this.invoiceId,
    required this.bank,
    required this.isp,
    required this.l10n,
    required this.onSuccess,
  });
  final String invoiceId;
  final BankAccountModel bank;
  final IspThemeColors isp;
  final AppLocalizations l10n;
  final VoidCallback onSuccess;

  @override
  ConsumerState<_UploadProofDialog> createState() => _UploadProofDialogState();
}

class _UploadProofDialogState extends ConsumerState<_UploadProofDialog> {
  PlatformFile? _selectedFile;
  String? _errorMessage;
  bool _uploading = false;

  Future<void> _pickFile() async {
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['jpg', 'jpeg', 'png', 'pdf'],
      );
      if (result == null || result.files.isEmpty) return;

      final file = result.files.first;
      // Max 10MB
      if ((file.size ?? 0) > 10 * 1024 * 1024) {
        setState(() => _errorMessage = 'Ukuran file maksimal 10MB');
        return;
      }
      setState(() {
        _selectedFile = file;
        _errorMessage = null;
      });
    } catch (e) {
      setState(() => _errorMessage = 'Gagal memilih file: $e');
    }
  }

  Future<void> _upload() async {
    if (_selectedFile == null) return;

    setState(() {
      _uploading = true;
      _errorMessage = null;
    });

    try {
      final file = _selectedFile!;
      final ext = file.name.split('.').last.toLowerCase();
      String contentType;
      switch (ext) {
        case 'jpg':
        case 'jpeg':
          contentType = 'image/jpeg';
          break;
        case 'png':
          contentType = 'image/png';
          break;
        case 'pdf':
          contentType = 'application/pdf';
          break;
        default:
          contentType = 'application/octet-stream';
      }

      final svc = ref.read(paymentServiceProvider);
      final res = await svc.submitPaymentProof(
        invoiceId: widget.invoiceId,
        filePath: file.path!,
        fileName: file.name,
        contentType: contentType,
      );

      if (!mounted) return;
      res.fold(
        (_) => widget.onSuccess(),
        (error) => setState(() => _errorMessage = error.message),
      );
    } catch (e) {
      if (mounted) {
        setState(() => _errorMessage = 'Gagal mengunggah: $e');
      }
    } finally {
      if (mounted) setState(() => _uploading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Upload Bukti Transfer'),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Bank info
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: widget.isp.surface,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Transfer ke ${widget.bank.bankName}',
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'No. Rekening: ${widget.bank.accountNumber}',
                    style: TextStyle(fontSize: 13, color: widget.isp.textSecondary),
                  ),
                  Text(
                    'Atas Nama: ${widget.bank.accountHolder}',
                    style: TextStyle(fontSize: 13, color: widget.isp.textSecondary),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
            // File picker
            OutlinedButton.icon(
              onPressed: _uploading ? null : _pickFile,
              icon: const Icon(Icons.attach_file),
              label: Text(_selectedFile == null ? 'Pilih File' : 'Ganti File'),
            ),
            const SizedBox(height: 8),
            // Selected file preview
            if (_selectedFile != null) ...[
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: widget.isp.accentSurface,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: widget.isp.accent.withOpacity(0.3)),
                ),
                child: Row(
                  children: [
                    Icon(
                      _selectedFile!.name.endsWith('.pdf')
                          ? Icons.picture_as_pdf
                          : Icons.image,
                      color: widget.isp.accent,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            _selectedFile!.name,
                            style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w500),
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            _formatBytes(_selectedFile!.size ?? 0),
                            style: TextStyle(fontSize: 11, color: widget.isp.textMuted),
                          ),
                        ],
                      ),
                    ),
                    if (!_uploading)
                      IconButton(
                        icon: const Icon(Icons.close, size: 16),
                        onPressed: () => setState(() => _selectedFile = null),
                        padding: EdgeInsets.zero,
                        constraints: const BoxConstraints(),
                      ),
                  ],
                ),
              ),
            ],
            // Error message
            if (_errorMessage != null) ...[
              const SizedBox(height: 8),
              Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: widget.isp.danger.withOpacity(0.1),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Row(
                  children: [
                    Icon(Icons.error_outline, size: 16, color: widget.isp.danger),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        _errorMessage!,
                        style: TextStyle(fontSize: 12, color: widget.isp.danger),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _uploading ? null : () => Navigator.pop(context),
          child: Text(widget.l10n.cancel),
        ),
        FilledButton(
          onPressed: (_selectedFile != null && !_uploading) ? _upload : null,
          child: _uploading
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Kirim'),
        ),
      ],
    );
  }

  String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
}
