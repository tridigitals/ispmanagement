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

// ─── Neubrutalist card ───────────────────────────────────────────

BoxDecoration _nbCard(IspThemeColors isp) => BoxDecoration(
      color: isp.surface,
      borderRadius: BorderRadius.circular(16),
      border: Border.all(color: isp.border, width: 1.5),
      boxShadow: [
        BoxShadow(
          color: isp.border.withOpacity(0.5),
          offset: const Offset(3, 3),
          blurRadius: 0,
        ),
      ],
    );

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
      final contentType = switch (ext) {
        'jpg' || 'jpeg' => 'image/jpeg',
        'png' => 'image/png',
        'pdf' => 'application/pdf',
        _ => 'application/octet-stream',
      };

      final svc = ref.read(paymentServiceProvider);
      final res = await svc.submitPaymentProof(
        invoiceId: widget.id,
        filePath: file.path!,
        fileName: file.name,
        contentType: contentType,
      );

      if (!mounted) return;
      res.fold(
        (_) => ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
              content: const Text('Bukti pembayaran berhasil diunggah'),
              backgroundColor: isp.success),
        ),
        (error) => ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
              content: Text('Gagal mengunggah: ${error.message}'),
              backgroundColor: isp.danger),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
            content: Text('Gagal memilih file: $e'),
            backgroundColor: isp.danger),
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
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 100),
          children: [
            // ── Hero card: receipt-style with left accent strip ──
            Container(
              decoration: _nbCard(isp),
              clipBehavior: Clip.antiAlias,
              child: IntrinsicHeight(
                child: Row(children: [
                  Container(
                    width: 5,
                    color: inv.isPaid
                        ? isp.success
                        : inv.isOverdue
                            ? isp.danger
                            : isp.warning,
                  ),
                  Expanded(
                    child: Padding(
                      padding: const EdgeInsets.all(20),
                      child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                                mainAxisAlignment:
                                    MainAxisAlignment.spaceBetween,
                                children: [
                                  Text(inv.invoiceNumber ?? '',
                                      style: TextStyle(
                                          color: isp.textSecondary,
                                          fontSize: 13,
                                          fontWeight: FontWeight.w500)),
                                  _StatusPill(
                                      isp: isp,
                                      label: inv.statusLabel(),
                                      isPaid: inv.isPaid,
                                      isOverdue: inv.isOverdue),
                                ]),
                            const SizedBox(height: 16),
                            Text(fmt.format(inv.amount),
                                style: TextStyle(
                                    fontSize: 32,
                                    fontWeight: FontWeight.w800,
                                    color: isp.textPrimary,
                                    height: 1.0)),
                            const SizedBox(height: 6),
                            Text('Jatuh tempo ${dateFmt.format(inv.dueDate)}',
                                style: TextStyle(
                                    color: isp.textSecondary, fontSize: 13)),
                          ]),
                    ),
                  ),
                ]),
              ),
            ),

            const SizedBox(height: 12),

            // ── Perforation line ──
            CustomPaint(
              size: const Size(double.infinity, 1),
              painter: _DashedLinePainter(color: isp.border),
            ),

            const SizedBox(height: 16),

            // ── Detail info ──
            Container(
              decoration: _nbCard(isp),
              padding: const EdgeInsets.all(16),
              child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Informasi Tagihan',
                        style: TextStyle(
                            fontSize: 13,
                            fontWeight: FontWeight.w700,
                            color: isp.textSecondary,
                            letterSpacing: 0.5)),
                    const SizedBox(height: 12),
                    _InfoRow(
                        isp: isp,
                        label: 'Jatuh tempo',
                        value: dateFmt.format(inv.dueDate)),
                    if (inv.paidAt != null)
                      _InfoRow(
                          isp: isp,
                          label: 'Dibayar pada',
                          value: dateFmt.format(inv.paidAt!)),
                    if (inv.subscriptionLabel != null)
                      _InfoRow(
                          isp: isp,
                          label: 'Layanan',
                          value: inv.subscriptionLabel!),
                    if (inv.notes != null && inv.notes!.isNotEmpty)
                      _InfoRow(isp: isp, label: 'Catatan', value: inv.notes!),
                  ]),
            ),

            const SizedBox(height: 16),

            // ── Action buttons ──
            if (!inv.isPaid) ...[
              _NeubrutalistBtn(
                icon: Icons.payment,
                label: 'Bayar Sekarang',
                isp: isp,
                filled: true,
                onTap: () => GoRouter.of(context).push('/payments/${inv.id}'),
              ),
              const SizedBox(height: 8),
              _NeubrutalistBtn(
                icon: Icons.upload_file,
                label: _uploadingProof ? 'Mengunggah...' : 'Upload Bukti',
                isp: isp,
                filled: false,
                loading: _uploadingProof,
                onTap: _uploadingProof ? null : _pickAndUploadProof,
              ),
              const SizedBox(height: 8),
              _NeubrutalistBtn(
                icon: Icons.help_outline,
                label: 'Bantuan',
                isp: isp,
                filled: false,
                onTap: () => GoRouter.of(context).push('/tickets/new'),
              ),
            ] else ...[
              _NeubrutalistBtn(
                icon: Icons.download,
                label: 'Download Struk',
                isp: isp,
                filled: false,
                onTap: () => _shareReceipt(context, inv),
              ),
              const SizedBox(height: 8),
              _NeubrutalistBtn(
                icon: Icons.print,
                label: 'Cetak Invoice',
                isp: isp,
                filled: false,
                onTap: () => _printReceipt(context, inv),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Future<void> _shareReceipt(BuildContext context, InvoiceModel inv) async {
    _showLoading(context);
    try {
      final path = await generateReceiptPdf(inv);
      if (!context.mounted) return;
      Navigator.of(context).pop();
      await Share.shareXFiles([XFile(path)],
          subject: 'Bukti Pembayaran ${inv.invoiceNumber}');
    } catch (e) {
      if (!context.mounted) return;
      try {
        Navigator.of(context).pop();
      } catch (_) {}
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
          content: Text('Gagal membuat PDF: $e'), backgroundColor: isp.danger));
    }
  }

  Future<void> _printReceipt(BuildContext context, InvoiceModel inv) async {
    _showLoading(context);
    try {
      final path = await generateReceiptPdf(inv);
      final bytes = await io.File(path).readAsBytes();
      if (!context.mounted) return;
      Navigator.of(context).pop();
      await Printing.layoutPdf(
          onLayout: (_) async => bytes, name: 'Invoice_${inv.invoiceNumber}');
    } catch (e) {
      if (!context.mounted) return;
      Navigator.of(context).pop();
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
          content: Text('Gagal mencetak: $e'), backgroundColor: isp.danger));
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
                  child: Column(mainAxisSize: MainAxisSize.min, children: [
                    CircularProgressIndicator(),
                    SizedBox(height: 16),
                    Text('Membuat PDF...')
                  ])))),
    );
  }
}

// ─── Status pill ─────────────────────────────────────────────────

class _StatusPill extends StatelessWidget {
  const _StatusPill(
      {required this.isp,
      required this.label,
      required this.isPaid,
      required this.isOverdue});
  final IspThemeColors isp;
  final String label;
  final bool isPaid;
  final bool isOverdue;

  @override
  Widget build(BuildContext context) {
    final color = isPaid
        ? isp.success
        : isOverdue
            ? isp.danger
            : isp.warning;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
          color: color.withOpacity(0.12),
          borderRadius: BorderRadius.circular(6)),
      child: Text(label,
          style: TextStyle(
              fontSize: 10, fontWeight: FontWeight.w700, color: color)),
    );
  }
}

// ─── Info row ────────────────────────────────────────────────────

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.isp, required this.label, required this.value});
  final IspThemeColors isp;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
        SizedBox(
            width: 110,
            child: Text(label,
                style: TextStyle(color: isp.textMuted, fontSize: 13))),
        Expanded(
            child: Text(value,
                style: TextStyle(
                    fontWeight: FontWeight.w600,
                    fontSize: 14,
                    color: isp.textPrimary))),
      ]),
    );
  }
}

// ─── Neubrutalist button ─────────────────────────────────────────

class _NeubrutalistBtn extends StatelessWidget {
  const _NeubrutalistBtn(
      {required this.icon,
      required this.label,
      required this.isp,
      required this.filled,
      this.loading = false,
      this.onTap});
  final IconData icon;
  final String label;
  final IspThemeColors isp;
  final bool filled;
  final bool loading;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: double.infinity,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(14),
          child: Container(
            padding: const EdgeInsets.symmetric(vertical: 14),
            decoration: BoxDecoration(
              color: filled ? isp.accent : isp.surface,
              borderRadius: BorderRadius.circular(14),
              border: Border.all(color: isp.border, width: 1.5),
              boxShadow: [
                BoxShadow(
                    color: isp.border.withOpacity(0.5),
                    offset: const Offset(3, 3),
                    blurRadius: 0)
              ],
            ),
            child: Center(
              child: loading
                  ? SizedBox(
                      width: 20,
                      height: 20,
                      child: CircularProgressIndicator(
                          strokeWidth: 2, color: isp.textPrimary))
                  : Row(mainAxisSize: MainAxisSize.min, children: [
                      Icon(icon,
                          size: 18,
                          color: filled ? Colors.white : isp.textPrimary),
                      const SizedBox(width: 8),
                      Text(label,
                          style: TextStyle(
                              fontSize: 14,
                              fontWeight: FontWeight.w700,
                              color: filled ? Colors.white : isp.textPrimary)),
                    ]),
            ),
          ),
        ),
      ),
    );
  }
}

// ─── Skeleton ────────────────────────────────────────────────────

class _InvoiceDetailSkeleton extends StatelessWidget {
  const _InvoiceDetailSkeleton();

  @override
  Widget build(BuildContext context) {
    return ListView(padding: const EdgeInsets.all(16), children: [
      const IspSkeletonCard(height: 160),
      const SizedBox(height: 16),
      IspCard(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          const IspShimmer.line(width: 140),
          const SizedBox(height: 12),
          ...List.generate(
              3,
              (_) => const Padding(
                  padding: EdgeInsets.symmetric(vertical: 6),
                  child: Row(children: [
                    IspShimmer.line(width: 100),
                    SizedBox(width: 12),
                    Expanded(child: IspShimmer.line())
                  ]))),
        ]),
      ),
      const SizedBox(height: 16),
      const IspShimmer.box(height: 48),
      const SizedBox(height: 8),
      const IspShimmer.box(height: 48),
    ]);
  }
}

// ─── Dashed line painter ─────────────────────────────────────────

class _DashedLinePainter extends CustomPainter {
  const _DashedLinePainter({required this.color});
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..strokeWidth = 1
      ..style = PaintingStyle.stroke;
    const dashWidth = 6.0;
    const dashGap = 4.0;
    var startX = 0.0;
    while (startX < size.width) {
      canvas.drawLine(Offset(startX, 0),
          Offset((startX + dashWidth).clamp(0, size.width), 0), paint);
      startX += dashWidth + dashGap;
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
