import 'dart:io';

import 'package:intl/intl.dart';
import 'package:path_provider/path_provider.dart';
import 'package:pdf/pdf.dart';
import 'package:pdf/widgets.dart' as pw;

import 'package:api_client/api_client.dart';

/// Generate a receipt PDF for a paid invoice and return the file path.
Future<String> generateReceiptPdf(InvoiceModel inv) async {
  final pdf = pw.Document();
  final dateFmt = DateFormat('d MMMM yyyy', 'id_ID');
  final currencyFmt = NumberFormat.currency(
    locale: 'id_ID',
    symbol: 'Rp',
    decimalDigits: 0,
  );

  pdf.addPage(
    pw.Page(
      pageFormat: PdfPageFormat.a4,
      margin: const pw.EdgeInsets.all(40),
      build: (context) => pw.Column(
        crossAxisAlignment: pw.CrossAxisAlignment.start,
        children: [
          // Header
          pw.Row(
            mainAxisAlignment: pw.MainAxisAlignment.spaceBetween,
            children: [
              pw.Column(
                crossAxisAlignment: pw.CrossAxisAlignment.start,
                children: [
                  pw.Text(
                    'TRIDIGITALS',
                    style: pw.TextStyle(
                      fontSize: 20,
                      fontWeight: pw.FontWeight.bold,
                      color: PdfColors.blue800,
                    ),
                  ),
                  pw.SizedBox(height: 4),
                  pw.Text(
                    'ISP Management',
                    style: const pw.TextStyle(
                      fontSize: 10,
                      color: PdfColors.grey600,
                    ),
                  ),
                ],
              ),
              pw.Column(
                crossAxisAlignment: pw.CrossAxisAlignment.end,
                children: [
                  pw.Text(
                    'BUKTI PEMBAYARAN',
                    style: pw.TextStyle(
                      fontSize: 14,
                      fontWeight: pw.FontWeight.bold,
                      color: PdfColors.green800,
                    ),
                  ),
                  pw.SizedBox(height: 4),
                  pw.Text(
                    'Status: LUNAS',
                    style: const pw.TextStyle(
                      fontSize: 10,
                      color: PdfColors.green700,
                    ),
                  ),
                ],
              ),
            ],
          ),
          pw.SizedBox(height: 8),
          pw.Divider(color: PdfColors.grey400),
          pw.SizedBox(height: 20),

          // Invoice info
          _infoRow('No. Invoice', inv.invoiceNumber),
          _infoRow('Tanggal Jatuh Tempo', dateFmt.format(inv.dueDate)),
          if (inv.paidAt != null)
            _infoRow('Tanggal Pembayaran', dateFmt.format(inv.paidAt!)),
          _infoRow('Tanggal Cetak', dateFmt.format(DateTime.now())),
          if (inv.subscriptionLabel != null)
            _infoRow('Layanan', inv.subscriptionLabel!),
          pw.SizedBox(height: 20),

          // Amount box
          pw.Container(
            width: double.infinity,
            padding: const pw.EdgeInsets.all(16),
            decoration: pw.BoxDecoration(
              color: PdfColors.grey100,
              borderRadius: pw.BorderRadius.circular(8),
            ),
            child: pw.Column(
              children: [
                pw.Text(
                  'TOTAL PEMBAYARAN',
                  style: const pw.TextStyle(
                    fontSize: 10,
                    color: PdfColors.grey600,
                  ),
                ),
                pw.SizedBox(height: 8),
                pw.Text(
                  currencyFmt.format(inv.amount),
                  style: pw.TextStyle(
                    fontSize: 28,
                    fontWeight: pw.FontWeight.bold,
                    color: PdfColors.blue900,
                  ),
                ),
                if (inv.amountPaid > 0 && inv.amountPaid != inv.amount) ...[
                  pw.SizedBox(height: 4),
                  pw.Text(
                    'Dibayar: ${currencyFmt.format(inv.amountPaid)}',
                    style: const pw.TextStyle(
                      fontSize: 12,
                      color: PdfColors.green700,
                    ),
                  ),
                ],
              ],
            ),
          ),
          pw.SizedBox(height: 20),

          // Detail table
          pw.TableHelper.fromTextArray(
            headerStyle: pw.TextStyle(
              fontWeight: pw.FontWeight.bold,
              fontSize: 10,
              color: PdfColors.white,
            ),
            headerDecoration: const pw.BoxDecoration(
              color: PdfColors.blue800,
            ),
            cellStyle: const pw.TextStyle(fontSize: 10),
            cellAlignment: pw.Alignment.centerLeft,
            headerAlignments: {
              0: pw.Alignment.centerLeft,
              1: pw.Alignment.centerRight
            },
            data: [
              ['Deskripsi', 'Jumlah'],
              [
                inv.subscriptionLabel ?? 'Tagihan ISP',
                currencyFmt.format(inv.amount),
              ],
              ['Status Pembayaran', inv.isPaid ? 'LUNAS' : inv.statusLabel()],
              ['Metode', 'Online Payment'],
            ],
          ),

          if (inv.notes != null && inv.notes!.isNotEmpty) ...[
            pw.SizedBox(height: 20),
            pw.Text(
              'Catatan:',
              style: pw.TextStyle(
                fontWeight: pw.FontWeight.bold,
                fontSize: 10,
              ),
            ),
            pw.SizedBox(height: 4),
            pw.Text(
              inv.notes!,
              style: const pw.TextStyle(fontSize: 10),
            ),
          ],

          pw.SizedBox(height: 40),
          pw.Divider(color: PdfColors.grey400),
          pw.SizedBox(height: 8),
          pw.Center(
            child: pw.Text(
              'Dokumen ini dicetak secara otomatis oleh sistem.',
              style: const pw.TextStyle(
                fontSize: 8,
                color: PdfColors.grey500,
              ),
            ),
          ),
          pw.SizedBox(height: 4),
          pw.Center(
            child: pw.Text(
              'Tridigitals ISP Management — api-isp-management.tridigitals.com',
              style: const pw.TextStyle(
                fontSize: 8,
                color: PdfColors.grey500,
              ),
            ),
          ),
        ],
      ),
    ),
  );

  // Save to temp directory
  final dir = await getTemporaryDirectory();
  final file = File(
    '${dir.path}/receipt_${inv.invoiceNumber.replaceAll(RegExp(r'[^a-zA-Z0-9]'), '_')}.pdf',
  );
  await file.writeAsBytes(await pdf.save());
  return file.path;
}

pw.Widget _infoRow(String label, String value) {
  return pw.Padding(
    padding: const pw.EdgeInsets.symmetric(vertical: 3),
    child: pw.Row(
      children: [
        pw.SizedBox(
          width: 140,
          child: pw.Text(
            label,
            style: const pw.TextStyle(
              fontSize: 10,
              color: PdfColors.grey600,
            ),
          ),
        ),
        pw.Text(
          ': ',
          style: const pw.TextStyle(fontSize: 10),
        ),
        pw.Expanded(
          child: pw.Text(
            value,
            style: pw.TextStyle(
              fontSize: 10,
              fontWeight: pw.FontWeight.bold,
            ),
          ),
        ),
      ],
    ),
  );
}
