import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../../l10n/app_localizations.dart';
import '../../../services/payment_providers.dart';

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
  int _secondsRemaining = 0;

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
    final l10n = AppLocalizations.of(context)!;
    final txnAsync = ref.watch(paymentStatusProvider(widget.transactionId));

    return Scaffold(
      appBar: AppBar(title: Text(l10n.paymentInstruction)),
      body: txnAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text(e.toString())),
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

class _Body extends StatelessWidget {
  const _Body({required this.txn, required this.invoiceId});
  final PaymentTransaction txn;
  final String invoiceId;

  @override
  Widget build(BuildContext context) {
    final fmt = NumberFormat.simpleCurrency(name: 'IDR', locale: 'id_ID');
    final l10n = AppLocalizations.of(context)!;
    if (txn.isPaid) {
      return _SuccessView(
        amount: fmt.format(txn.amount),
        onContinue: () => context.go('/invoices/$invoiceId'),
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
              color: IspColors.warning.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(IspRadii.lg),
            ),
            child: Column(
              children: [
                Text(
                  l10n.totalPayment,
                  style: const TextStyle(color: IspColors.textTertiary),
                ),
                const SizedBox(height: 4),
                Text(
                  fmt.format(txn.amount),
                  style: const TextStyle(
                    fontSize: 28,
                    fontWeight: FontWeight.w800,
                  ),
                ),
                const SizedBox(height: 12),
                if (txn.expiredAt != null)
                  _Countdown(expiresAt: txn.expiredAt!),
                const SizedBox(height: 8),
                IspStatusBadge(
                  label: txn.statusLabel,
                  tone: txn.isPending
                      ? StatusTone.warning
                      : txn.isExpired
                          ? StatusTone.danger
                          : StatusTone.neutral,
                ),
              ],
            ),
          ),
          const SizedBox(height: IspSpacing.lg),
          // Payment-specific UI.
          if (txn.method == PaymentMethod.qris && txn.qrCodeUrl != null)
            _QrisView(qrUrl: txn.qrCodeUrl!)
          else if (txn.method == PaymentMethod.virtualAccount &&
              txn.vaNumber != null)
            _VirtualAccountView(vaNumber: txn.vaNumber!)
          else if (txn.method == PaymentMethod.ewallet &&
              txn.paymentUrl != null)
            _EWalletView(
              paymentUrl: txn.paymentUrl!,
              actions: txn.actions ?? const [],
            )
          else if (txn.paymentCode != null)
            _PaymentCodeView(code: txn.paymentCode!)
          else
            _GenericView(
              message:
                  'Selesaikan pembayaran di aplikasi ${txn.method.label} Anda',
            ),
          const SizedBox(height: IspSpacing.lg),
          OutlinedButton(
            onPressed: () => context.go('/invoices/$invoiceId'),
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
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            const Text(
              'Scan QRIS dengan e-wallet Anda',
              style: TextStyle(fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 16),
            Container(
              padding: const EdgeInsets.all(16),
              color: Colors.white,
              child: QrImageView(
                data: qrUrl,
                version: QrVersions.auto,
                size: 240,
                errorCorrectionLevel: QrErrorCorrectLevel.M,
              ),
            ),
            const SizedBox(height: 12),
            const Text(
              'Berlaku untuk semua aplikasi e-wallet dan mobile banking',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 12, color: IspColors.textTertiary),
            ),
          ],
        ),
      ),
    );
  }
}

class _VirtualAccountView extends StatelessWidget {
  const _VirtualAccountView({required this.vaNumber});
  final String vaNumber;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            const Text('Nomor Virtual Account',
                style: TextStyle(fontWeight: FontWeight.w600)),
            const SizedBox(height: 16),
            SelectableText(
              vaNumber,
              style: const TextStyle(
                fontSize: 24,
                fontWeight: FontWeight.w800,
                letterSpacing: 1.5,
              ),
            ),
            const SizedBox(height: 12),
            const Text(
              'Lakukan transfer ke nomor VA di atas melalui mobile banking atau ATM. '
              'Pembayaran akan otomatis terdeteksi dalam 1-2 menit.',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 12, color: IspColors.textTertiary),
            ),
          ],
        ),
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
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            const Icon(Icons.account_balance_wallet, size: 48),
            const SizedBox(height: 16),
            const Text(
              'Selesaikan pembayaran di aplikasi e-wallet',
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 16),
            for (final action in actions)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
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
      ),
    );
  }
}

class _PaymentCodeView extends StatelessWidget {
  const _PaymentCodeView({required this.code});
  final String code;
  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            const Text('Kode Pembayaran',
                style: TextStyle(fontWeight: FontWeight.w600)),
            const SizedBox(height: 12),
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
      ),
    );
  }
}

class _GenericView extends StatelessWidget {
  const _GenericView({required this.message});
  final String message;
  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Center(
          child: Text(message, textAlign: TextAlign.center),
        ),
      ),
    );
  }
}

class _Countdown extends StatelessWidget {
  const _Countdown({required this.expiresAt});
  final DateTime expiresAt;

  @override
  Widget build(BuildContext context) {
    return StreamBuilder<int>(
      stream: Stream.periodic(const Duration(seconds: 1), (_) {
        return expiresAt.difference(DateTime.now()).inSeconds;
      }),
      initialData: expiresAt.difference(DateTime.now()).inSeconds,
      builder: (context, snap) {
        final sec = snap.data ?? 0;
        if (sec <= 0) {
          return const Text(
            'Batas waktu habis',
            style: TextStyle(color: IspColors.danger, fontWeight: FontWeight.w600),
          );
        }
        final h = (sec ~/ 3600).toString().padLeft(2, '0');
        final m = ((sec % 3600) ~/ 60).toString().padLeft(2, '0');
        final s = (sec % 60).toString().padLeft(2, '0');
        return Text(
          'Batas waktu: $h:$m:$s',
          style: const TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w600,
            color: IspColors.warning,
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
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(IspSpacing.xl),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              padding: const EdgeInsets.all(IspSpacing.lg),
              decoration: const BoxDecoration(
                color: IspColors.success,
                shape: BoxShape.circle,
              ),
              child: const Icon(Icons.check, color: Colors.white, size: 64),
            ),
            const SizedBox(height: IspSpacing.lg),
            const Text(
              'Pembayaran Berhasil!',
              style: TextStyle(fontSize: 22, fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 8),
            Text(
              amount,
              style: const TextStyle(
                fontSize: 28,
                fontWeight: FontWeight.w800,
                color: IspColors.success,
              ),
            ),
            const SizedBox(height: 24),
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
