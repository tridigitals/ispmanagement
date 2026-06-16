import 'dart:io' as io;

import 'package:api_client/api_client.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';
import 'package:printing/printing.dart';
import 'package:share_plus/share_plus.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../services/missing_providers.dart';
import '../../services/feature_providers.dart';
import '../../utils/loading_skeleton.dart';
import '../../utils/receipt_pdf.dart';

class InvoiceDetailScreen extends ConsumerStatefulWidget {
  const InvoiceDetailScreen({required this.id, super.key});
  final String id;

  @override
  ConsumerState<InvoiceDetailScreen> createState() =>
      _InvoiceDetailScreenState();
}

class _InvoiceDetailScreenState extends ConsumerState<InvoiceDetailScreen> {

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
        invoiceId: widget.id,
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
    final dateFmt = DateFormat('d MMMM yyyy', 'id_ID');
    final invAsync = ref.watch(invoiceByIdProvider(widget.id));

    return Scaffold(
      appBar: AppBar(title: const Text('Detail Tagihan')),
      body: invAsync.when(
        loading: () => const _InvoiceDetailSkeleton(),
        error: (e, _) => IspErrorState(
          message: e.toString(),
          onRetry: () => ref.invalidate(invoiceByIdProvider(widget.id)),
        ),
        data: (inv) => ListView(
          padding: const EdgeInsets.all(IspSpacing.lg),
          children: [
            // ── Hero card ──
            Container(
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: inv.isPaid
                      ? [IspColors.success, IspColors.success.withOpacity(0.85)]
                      : inv.isOverdue
                          ? [IspColors.danger, IspColors.danger.withOpacity(0.85)]
                          : [IspColors.primary, IspColors.info],
                ),
                borderRadius: BorderRadius.circular(IspRadii.xl),
                boxShadow: IspShadows.md,
              ),
              padding: const EdgeInsets.all(IspSpacing.xl),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        inv.invoiceNumber,
                        style: const TextStyle(
                          color: Colors.white70,
                          fontSize: 14,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                      IspStatusBadge(
                        label: inv.statusLabel(),
                        tone: inv.isPaid
                            ? StatusTone.success
                            : inv.isOverdue
                                ? StatusTone.danger
                                : StatusTone.warning,
                      ),
                    ],
                  ),
                  const SizedBox(height: IspSpacing.xl),
                  Text(
                    fmt.format(inv.amount),
                    style: const TextStyle(
                      fontSize: 32,
                      fontWeight: FontWeight.w800,
                      color: Colors.white,
                      height: 1.0,
                    ),
                  ),
                  const SizedBox(height: IspSpacing.sm),
                  Text(
                    'Jatuh tempo ${dateFmt.format(inv.dueDate)}',
                    style: const TextStyle(color: Colors.white60, fontSize: 13),
                  ),
                ],
              ),
            ),
            const SizedBox(height: IspSpacing.lg),

            // ── Details card ──
            IspCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Informasi Tagihan',
                    style: TextStyle(
                      fontSize: 14,
                      fontWeight: FontWeight.w600,
                      color: isp.textSecondary,
                    ),
                  ),
                  const SizedBox(height: IspSpacing.md),
                  _InfoRow(
                      label: 'Jatuh tempo', value: dateFmt.format(inv.dueDate)),
                  if (inv.paidAt != null)
                    _InfoRow(
                        label: 'Dibayar pada',
                        value: dateFmt.format(inv.paidAt!)),
                  if (inv.subscriptionLabel != null)
                    _InfoRow(label: 'Layanan', value: inv.subscriptionLabel!),
                  if (inv.notes != null && inv.notes!.isNotEmpty)
                    _InfoRow(label: 'Catatan', value: inv.notes!),
                ],
              ),
            ),
            const SizedBox(height: IspSpacing.lg),

            // ── Actions ──
            if (!inv.isPaid) ...[
              ElevatedButton.icon(
                onPressed: () =>
                    GoRouter.of(context).push('/payments/${inv.id}'),
                icon: const Icon(Icons.payment),
                label: const Text('Bayar Sekarang'),
              ),
              const SizedBox(height: IspSpacing.sm),
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
              OutlinedButton.icon(
                onPressed: () => GoRouter.of(context).push('/tickets/new'),
                icon: const Icon(Icons.help_outline),
                label: const Text('Butuh Bantuan?'),
              ),
            ] else ...[
              OutlinedButton.icon(
                onPressed: () => _shareReceipt(context, inv),
                icon: const Icon(Icons.download),
                label: const Text('Download Struk'),
              ),
              const SizedBox(height: IspSpacing.sm),
              OutlinedButton.icon(
                onPressed: () => _printReceipt(context, inv),
                icon: const Icon(Icons.print),
                label: const Text('Cetak Invoice'),
              ),
            ],
          ],
        ),
      ),
    );
  }

  /// Generate PDF → share via system share sheet (save to Files, Drive, WhatsApp, etc.)
  Future<void> _shareReceipt(BuildContext context, InvoiceModel inv) async {
    _showLoading(context);
    try {
      final path = await generateReceiptPdf(inv);
      if (!context.mounted) return;
      Navigator.of(context).pop(); // dismiss loading

      await Share.shareXFiles(
        [XFile(path)],
        subject: 'Bukti Pembayaran ${inv.invoiceNumber}',
      );
    } catch (e) {
      if (!context.mounted) return;
      try { Navigator.of(context).pop(); } catch (_) {}
      _showError(context, 'Gagal membuat PDF: $e');
    }
  }

  /// Generate PDF → open system print dialog
  Future<void> _printReceipt(BuildContext context, InvoiceModel inv) async {
    _showLoading(context);
    try {
      final path = await generateReceiptPdf(inv);
      final bytes = await io.File(path).readAsBytes();
      if (!context.mounted) return;
      Navigator.of(context).pop();

      await Printing.layoutPdf(
        onLayout: (_) async => bytes,
        name: 'Invoice_${inv.invoiceNumber}',
      );
    } catch (e) {
      if (!context.mounted) return;
      Navigator.of(context).pop();
      _showError(context, 'Gagal mencetak: $e');
    }
  }

  void _showLoading(BuildContext context) {
    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (_) => const Center(
        child: Card(
          child: Padding(
            padding: EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                CircularProgressIndicator(),
                SizedBox(height: 16),
                Text('Membuat PDF...'),
              ],
            ),
          ),
        ),
      ),
    );
  }

  void _showError(BuildContext context, String msg) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(msg), backgroundColor: isp.danger),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});
  final String label;
  final String value;
  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return Padding(
      padding: const EdgeInsets.symmetric(vertical: IspSpacing.sm),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 120,
            child: Text(
              label,
              style:
                  TextStyle(color: isp.textMuted, fontSize: 13),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: const TextStyle(fontWeight: FontWeight.w500, fontSize: 14),
            ),
          ),
        ],
      ),
    );
  }
}

/// Skeleton loading state for invoice detail.
class _InvoiceDetailSkeleton extends StatelessWidget {
  const _InvoiceDetailSkeleton();

  @override
  Widget build(BuildContext context) {

final isp = context.isp;
return ListView(
      padding: const EdgeInsets.all(IspSpacing.lg),
      children: [
        // Hero card skeleton
        const IspSkeletonCard(height: 160),
        const SizedBox(height: IspSpacing.lg),
        // Details card skeleton
        IspCard(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const IspShimmer.line(width: 140),
              const SizedBox(height: IspSpacing.md),
              ...List.generate(
                3,
                (_) => const Padding(
                  padding: EdgeInsets.symmetric(vertical: IspSpacing.sm),
                  child: Row(
                    children: [
                      IspShimmer.line(width: 100),
                      SizedBox(width: IspSpacing.md),
                      Expanded(child: IspShimmer.line()),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: IspSpacing.lg),
        const IspShimmer.box(height: 48),
        const SizedBox(height: IspSpacing.sm),
        const IspShimmer.box(height: 48),
      ],
    );
  }
}
