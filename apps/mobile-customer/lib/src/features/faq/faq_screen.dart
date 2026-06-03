import 'package:flutter/material.dart';
import 'package:ui_kit/ui_kit.dart';

/// In-app FAQ — for ISP customers, the most common questions.
/// Backend may serve dynamic FAQ via `GET /api/portal/faq` later.
class _FaqItem {
  const _FaqItem({
    required this.question,
    required this.answer,
    this.category = 'Umum',
  });
  final String question;
  final String answer;
  final String category;
}

const _faqItems = <_FaqItem>[
  _FaqItem(
    question: 'Bagaimana cara bayar tagihan?',
    answer:
        'Buka halaman Tagihan, pilih tagihan yang akan dibayar, lalu pilih metode pembayaran '
        '(Virtual Account, E-Wallet, QRIS, atau Kartu Kredit) dan ikuti instruksi yang muncul.',
    category: 'Pembayaran',
  ),
  _FaqItem(
    question: 'Apa yang harus saya lakukan jika koneksi internet lambat?',
    answer:
        'Coba restart router ONT Anda dengan mencabut listrik selama 30 detik. '
        'Jika masih lambat, lakukan Speed Test di aplikasi dan laporkan gangguan via menu Bantuan.',
    category: 'Koneksi',
  ),
  _FaqItem(
    question: 'Kapan tagihan saya terbit?',
    answer:
        'Tagihan terbit otomatis 7 hari sebelum tanggal jatuh tempo. '
        'Anda akan menerima notifikasi push dan email.',
    category: 'Tagihan',
  ),
  _FaqItem(
    question: 'Bagaimana jika saya lupa password?',
    answer:
        'Di halaman Login, ketuk "Lupa kata sandi?" lalu masukkan email Anda. '
        'Link reset akan dikirim ke email Anda.',
    category: 'Akun',
  ),
  _FaqItem(
    question: 'Berapa lama pemasangan baru?',
    answer:
        'Pemasangan baru biasanya memakan waktu 1-3 hari kerja setelah konfirmasi alamat, '
        'tergantung antrian teknisi di area Anda.',
    category: 'Layanan',
  ),
  _FaqItem(
    question: 'Apakah saya bisa upgrade paket?',
    answer:
        'Untuk saat ini, permintaan upgrade paket dilakukan melalui Customer Service. '
        'Dalam waktu dekat fitur ini akan tersedia di aplikasi.',
    category: 'Layanan',
  ),
  _FaqItem(
    question: 'Apa yang terjadi jika saya telat bayar?',
    answer:
        'Setelah tanggal jatuh tempo, Anda masuk masa tenggang (grace period) 7 hari. '
        'Jika masih belum dibayar, koneksi akan dinonaktifkan sementara dan dapat diaktifkan kembali setelah pembayaran lunas.',
    category: 'Tagihan',
  ),
  _FaqItem(
    question: 'Bagaimana cara ganti password WiFi?',
    answer:
        'Login ke router Anda melalui 192.168.1.1 dengan akun admin (default di stiker belakang router). '
        'Masuk ke menu WLAN/Wireless, ubah SSID dan password sesuai keinginan.',
    category: 'Koneksi',
  ),
];

class FaqScreen extends StatefulWidget {
  const FaqScreen({super.key});

  @override
  State<FaqScreen> createState() => _FaqScreenState();
}

class _FaqScreenState extends State<FaqScreen> {
  String _filter = 'Semua';
  String _search = '';

  List<_FaqItem> get _filtered {
    return _faqItems.where((item) {
      if (_filter != 'Semua' && item.category != _filter) return false;
      if (_search.isEmpty) return true;
      final q = _search.toLowerCase();
      return item.question.toLowerCase().contains(q) ||
          item.answer.toLowerCase().contains(q);
    }).toList();
  }

  @override
  Widget build(BuildContext context) {
    final categories = ['Semua', ..._faqItems.map((i) => i.category).toSet()];
    return Scaffold(
      appBar: AppBar(title: const Text('FAQ')),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: TextField(
              decoration: const InputDecoration(
                hintText: 'Cari pertanyaan...',
                prefixIcon: Icon(Icons.search),
              ),
              onChanged: (v) => setState(() => _search = v),
            ),
          ),
          SizedBox(
            height: 40,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              scrollDirection: Axis.horizontal,
              itemBuilder: (_, i) {
                final cat = categories[i];
                return FilterChip(
                  label: Text(cat),
                  selected: _filter == cat,
                  onSelected: (_) => setState(() => _filter = cat),
                );
              },
              separatorBuilder: (_, __) => const SizedBox(width: 8),
              itemCount: categories.length,
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: _filtered.isEmpty
                ? const Center(child: Text('Tidak ada hasil'))
                : ListView.separated(
                    padding: const EdgeInsets.all(16),
                    itemBuilder: (_, i) => _FaqTile(item: _filtered[i]),
                    separatorBuilder: (_, __) => const SizedBox(height: 8),
                    itemCount: _filtered.length,
                  ),
          ),
        ],
      ),
    );
  }
}

class _FaqTile extends StatefulWidget {
  const _FaqTile({required this.item});
  final _FaqItem item;
  @override
  State<_FaqTile> createState() => _FaqTileState();
}

class _FaqTileState extends State<_FaqTile> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: InkWell(
        borderRadius: BorderRadius.circular(IspRadii.lg),
        onTap: () => setState(() => _expanded = !_expanded),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(
                      widget.item.question,
                      style: const TextStyle(
                        fontSize: 14,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  Icon(
                    _expanded ? Icons.expand_less : Icons.expand_more,
                    color: IspColors.textTertiary,
                  ),
                ],
              ),
              if (_expanded) ...[
                const SizedBox(height: 12),
                Text(
                  widget.item.answer,
                  style: const TextStyle(
                    fontSize: 13,
                    color: IspColors.textSecondary,
                    height: 1.5,
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
