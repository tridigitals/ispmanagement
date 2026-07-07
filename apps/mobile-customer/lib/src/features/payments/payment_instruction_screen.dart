import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/feature_providers.dart';
import '../../services/payment_providers.dart';
import '../../utils/loading_skeleton.dart';

/// Active payment screen — shows VA number / QR / redirect URL with auto-poll.
class PaymentInstructionScreen extends ConsumerStatefulWidget {
  const PaymentInstructionScreen({
    required this.invoiceId,
    required this.transactionId,
    super.key,
  });
  final String invoiceId;
  final String transactionId;

  @override
  ConsumerState<PaymentInstructionScreen> createState() =>
      _PaymentInstructionScreenState();
}

class _PaymentInstructionScreenState
    extends ConsumerState<PaymentInstructionScreen> {
  Timer? _pollTimer;

  late final IspThemeColors isp;



  @override


  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }

  @override
  void initState() {
    super.initState();
    // Poll every 5s for status updates.
    _pollTimer = Timer.periodic(const Duration(seconds: 5), (_) {
      ref.invalidate(paymentStatusProvider(widget.transactionId));
    });
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
final l10n = AppLocalizations.of(context);
    final txnAsync = ref.watch(paymentStatusProvider(widget.transactionId));

    return Scaffold(
      appBar: AppBar(title: Text(l10n.paymentInstruction)),
      body: txnAsync.when(
        loading: () => const _PaymentInstructionSkeleton(),
        error: (e, _) => IspErrorState(
          message: e.toString(),
          onRetry: () =>
              ref.invalidate(paymentStatusProvider(widget.transactionId)),
        ),
        data: (txn) {
          // Stop polling once terminal state.
          if (txn.isPaid || txn.isExpired || txn.isFailed) {
            _pollTimer?.cancel();
          }
          return _Body(
            txn: txn,
            invoiceId: widget.invoiceId,
          );
        },
      ),
    );
  }
}

class _Body extends ConsumerStatefulWidget {
  const _Body({required this.txn, required this.invoiceId});
  final PaymentTransaction txn;
  final String invoiceId;

  @override
  ConsumerState<_Body> createState() => _BodyState();
}

class _BodyState extends ConsumerState<_Body> {

  late final IspThemeColors isp;



  @override


  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }
  bool _uploadingProof = false;

  Future<void> _pickAndUploadProof() async {
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['jpg', 'jpeg', 'png', 'pdf'],
      );
      if (result == null || result.files.isEmpty) return;
      final file = result.files.first;
      if (file.path == null) return;

      setState(() => _uploadingProof = true);

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
        (_) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Bukti pembayaran berhasil diunggah'),
              backgroundColor: isp.success,
            ),
          );
        },
        (error) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Gagal mengunggah: ${error.message}'),
              backgroundColor: isp.danger,
            ),
          );
        },
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Gagal memilih file: $e'),
          backgroundColor: isp.danger,
        ),
      );
    } finally {
      if (mounted) setState(() => _uploadingProof = false);
    }
  }

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final l10n = AppLocalizations.of(context);
    if (widget.txn.isPaid) {
      return _SuccessView(
        amount: fmt.format(widget.txn.amount),
        onContinue: () => context.go('/invoices/${widget.invoiceId}'),
      );
    }
    return SingleChildScrollView(
      padding: const EdgeInsets.all(IspSpacing.lg),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Status header.
          Container(
            padding: const EdgeInsets.all(IspSpacing.lg),
            decoration: BoxDecoration(
              color: isp.warning.withOpacity(0.1),
              borderRadius: BorderRadius.circular(IspRadii.lg),
              border: Border.all(color: isp.border, width: 1.5),
              boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
            ),
            child: Column(
              children: [
                Text(
                  l10n.totalPayment,
                  style: TextStyle(color: isp.textMuted),
                ),
                const SizedBox(height: IspSpacing.xs),
                Text(
                  fmt.format(widget.txn.amount),
                  style: const TextStyle(
                    fontSize: 28,
                    fontWeight: FontWeight.w800,
                  ),
                ),
                const SizedBox(height: IspSpacing.md),
                if (widget.txn.expiredAt != null)
                  _Countdown(expiresAt: widget.txn.expiredAt!),
                const SizedBox(height: IspSpacing.sm),
                IspStatusBadge(
                  label: widget.txn.statusLabel,
                  tone: widget.txn.isPending
                      ? StatusTone.warning
                      : widget.txn.isExpired
                          ? StatusTone.danger
                          : StatusTone.neutral,
                ),
              ],
            ),
          ),
          const SizedBox(height: IspSpacing.lg),
          // Payment-specific UI.
          if (widget.txn.method == PaymentMethod.qris &&
              widget.txn.qrCodeUrl != null)
            _QrisView(qrUrl: widget.txn.qrCodeUrl!)
          else if (widget.txn.method == PaymentMethod.virtualAccount &&
              widget.txn.vaNumber != null)
            _VirtualAccountView(vaNumber: widget.txn.vaNumber!)
          else if (widget.txn.method == PaymentMethod.ewallet &&
              widget.txn.paymentUrl != null)
            _EWalletView(
              paymentUrl: widget.txn.paymentUrl!,
              actions: widget.txn.actions ?? const [],
            )
          else if (widget.txn.paymentCode != null)
            _PaymentCodeView(code: widget.txn.paymentCode!)
          else
            _GenericView(
              message:
                  'Selesaikan pembayaran di aplikasi ${widget.txn.method.label} Anda',
            ),
          const SizedBox(height: IspSpacing.lg),
          // Upload payment proof button
          OutlinedButton.icon(
            onPressed: _uploadingProof ? null : _pickAndUploadProof,
            icon: _uploadingProof
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.upload_file),
            label: Text(
              _uploadingProof ? 'Mengunggah...' : 'Upload Bukti Pembayaran',
            ),
          ),
          const SizedBox(height: IspSpacing.sm),
          OutlinedButton(
            onPressed: () => context.go('/invoices/${widget.invoiceId}'),
            child: const Text('Saya sudah bayar'),
          ),
        ],
      ),
    );
  }
}

class _QrisView extends StatelessWidget {
  const _QrisView({required this.qrUrl});
  final String qrUrl;

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return Container(decoration: NbStyle.card(context), child:
      child: Column(
        children: [
          const Text(
            'Scan QRIS dengan e-wallet Anda',
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: IspSpacing.lg),
          Container(
            padding: const EdgeInsets.all(IspSpacing.lg),
            decoration: BoxDecoration(
              color: Colors.white,
              borderRadius: BorderRadius.circular(IspRadii.md),
            ),
            child: QrImageView(
              data: qrUrl,
              version: QrVersions.auto,
              size: 240,
              errorCorrectionLevel: QrErrorCorrectLevel.M,
            ),
          ),
          const SizedBox(height: IspSpacing.md),
          Text(
            'Berlaku untuk semua aplikasi e-wallet dan mobile banking',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 12, color: isp.textMuted),
          ),
        ],
      ),
    );
  }
}

class _VirtualAccountView extends StatelessWidget {
  const _VirtualAccountView({required this.vaNumber});
  final String vaNumber;

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return Container(decoration: NbStyle.card(context), child:
      child: Column(
        children: [
          const Text(
            'Nomor Virtual Account',
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: IspSpacing.lg),
          SelectableText(
            vaNumber,
            style: const TextStyle(
              fontSize: 24,
              fontWeight: FontWeight.w800,
              letterSpacing: 1.5,
            ),
          ),
          const SizedBox(height: IspSpacing.md),
          Text(
            'Lakukan transfer ke nomor VA di atas melalui mobile banking atau ATM. '
            'Pembayaran akan otomatis terdeteksi dalam 1-2 menit.',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 12, color: isp.textMuted),
          ),
        ],
      ),
    );
  }
}

class _EWalletView extends StatelessWidget {
  const _EWalletView({required this.paymentUrl, required this.actions});
  final String paymentUrl;
  final List<PaymentAction> actions;

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return Container(decoration: NbStyle.card(context), child:
      child: Column(
        children: [
          const Icon(Icons.account_balance_wallet, size: 48),
          const SizedBox(height: IspSpacing.lg),
          const Text(
            'Selesaikan pembayaran di aplikasi e-wallet',
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: IspSpacing.lg),
          for (final action in actions)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: IspSpacing.xs),
              child: ElevatedButton(
                onPressed: () => launchUrl(
                  Uri.parse(action.url),
                  mode: LaunchMode.externalApplication,
                ),
                child: Text('Buka ${action.name}'),
              ),
            ),
          if (actions.isEmpty)
            ElevatedButton(
              onPressed: () => launchUrl(
                Uri.parse(paymentUrl),
                mode: LaunchMode.externalApplication,
              ),
              child: const Text('Buka aplikasi'),
            ),
        ],
      ),
    );
  }
}

class _PaymentCodeView extends StatelessWidget {
  const _PaymentCodeView({required this.code});
  final String code;
  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return Container(decoration: NbStyle.card(context), child:
      child: Column(
        children: [
          const Text(
            'Kode Pembayaran',
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: IspSpacing.md),
          SelectableText(
            code,
            style: const TextStyle(
              fontSize: 22,
              fontWeight: FontWeight.w800,
              letterSpacing: 1.2,
            ),
          ),
        ],
      ),
    );
  }
}

class _GenericView extends StatelessWidget {
  const _GenericView({required this.message});
  final String message;
  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return IspCard(
      nbStyle: true,
      child: Center(
        child: Text(message, textAlign: TextAlign.center),
      ),
    );
  }
}

class _Countdown extends StatelessWidget {
  const _Countdown({required this.expiresAt});
  final DateTime expiresAt;

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return StreamBuilder<int>(
      stream: Stream.periodic(const Duration(seconds: 1), (_) {
        return expiresAt.difference(DateTime.now()).inSeconds;
      }),
      initialData: expiresAt.difference(DateTime.now()).inSeconds,
      builder: (context, snap) {
        final sec = snap.data ?? 0;
        if (sec <= 0) {
          return Text(
            'Batas waktu habis',
            style:
                TextStyle(color: isp.danger, fontWeight: FontWeight.w600),
          );
        }
        final h = (sec ~/ 3600).toString().padLeft(2, '0');
        final m = ((sec % 3600) ~/ 60).toString().padLeft(2, '0');
        final s = (sec % 60).toString().padLeft(2, '0');
        return Text(
          'Batas waktu: $h:$m:$s',
          style: TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w600,
            color: isp.warning,
          ),
        );
      },
    );
  }
}

class _SuccessView extends StatelessWidget {
  const _SuccessView({required this.amount, required this.onContinue});
  final String amount;
  final VoidCallback onContinue;
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
                color: isp.success,
                shape: BoxShape.circle,
              ),
              child: const Icon(Icons.check, color: Color(0xFFFFFFFF), size: 64),
            ),
            const SizedBox(height: IspSpacing.lg),
            const Text(
              'Pembayaran Berhasil!',
              style: TextStyle(fontSize: 22, fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: IspSpacing.sm),
            Text(
              amount,
              style: TextStyle(
                fontSize: 28,
                fontWeight: FontWeight.w800,
                color: isp.success,
              ),
            ),
            const SizedBox(height: IspSpacing.xl),
            ElevatedButton(
              onPressed: onContinue,
              child: const Text('Lihat Tagihan'),
            ),
          ],
        ),
      ),
    );
  }
}

/// Skeleton loading state for payment instruction screen.
class _PaymentInstructionSkeleton extends StatelessWidget {
  const _PaymentInstructionSkeleton();

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return ListView(
      padding: const EdgeInsets.all(IspSpacing.lg),
      children: [
        // Status header skeleton
        Container(
          padding: const EdgeInsets.all(IspSpacing.lg),
          decoration: BoxDecoration(
            color: isp.surface,
            borderRadius: BorderRadius.circular(IspRadii.lg),
          ),
          child: const Column(
            children: [
              IspShimmer.line(width: 100),
              SizedBox(height: IspSpacing.xs),
              IspShimmer.line(width: 200, height: 28),
              SizedBox(height: IspSpacing.md),
              IspShimmer.line(width: 140),
              SizedBox(height: IspSpacing.sm),
              IspShimmer.line(width: 80, height: 24),
            ],
          ),
        ),
        const SizedBox(height: IspSpacing.lg),
        // Payment method card skeleton
        const IspSkeletonCard(height: 280),
        const SizedBox(height: IspSpacing.lg),
        // Button skeleton
        const IspShimmer.box(height: 48),
      ],
    );
  }
}
