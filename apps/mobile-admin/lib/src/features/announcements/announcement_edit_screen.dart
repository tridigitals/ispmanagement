import 'package:flutter/material.dart';

class AnnouncementEditScreen extends StatefulWidget {
  final String? announcementId;
  const AnnouncementEditScreen({super.key, this.announcementId});

  @override
  State<AnnouncementEditScreen> createState() => _AnnouncementEditScreenState();
}

class _AnnouncementEditScreenState extends State<AnnouncementEditScreen> {
  final _titleCtrl = TextEditingController();
  final _bodyCtrl = TextEditingController();
  String _audience = 'all';
  bool _pinned = false;

  @override
  void dispose() {
    _titleCtrl.dispose();
    _bodyCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final isEdit = widget.announcementId != null;

    return Scaffold(
      appBar: AppBar(
        title: Text(isEdit ? 'Edit Pengumuman' : 'Buat Pengumuman'),
        actions: [
          TextButton(
            onPressed: () {},
            child: const Text('Simpan'),
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          TextField(
            controller: _titleCtrl,
            decoration: const InputDecoration(labelText: 'Judul'),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _bodyCtrl,
            maxLines: 5,
            decoration: const InputDecoration(
              labelText: 'Isi pengumuman',
              alignLabelWithHint: true,
            ),
          ),
          const SizedBox(height: 16),
          Text('Target Audiens', style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 8),
          SegmentedButton<String>(
            segments: const [
              ButtonSegment(value: 'all', label: Text('Semua')),
              ButtonSegment(value: 'active', label: Text('Aktif')),
              ButtonSegment(value: 'expired', label: Text('Expired')),
            ],
            selected: {_audience},
            onSelectionChanged: (s) => setState(() => _audience = s.first),
          ),
          const SizedBox(height: 16),
          SwitchListTile(
            title: const Text('Pin di atas'),
            subtitle: const Text('Tampilkan di banner utama'),
            value: _pinned,
            onChanged: (v) => setState(() => _pinned = v),
          ),
        ],
      ),
    );
  }
}
