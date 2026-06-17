import 'package:api_client/api_client.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:image_picker/image_picker.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';

/// Source for ticket attachments — mirrors the profile-upload pattern so
/// camera permission flow is consistent across the app.
enum _AttachmentSource { camera, files }

class NewTicketScreen extends ConsumerStatefulWidget {
  const NewTicketScreen({super.key});

  @override
  ConsumerState<NewTicketScreen> createState() => _NewTicketScreenState();
}

class _NewTicketScreenState extends ConsumerState<NewTicketScreen> {
  final _formKey = GlobalKey<FormState>();
  final _subjectCtrl = TextEditingController();
  final _descriptionCtrl = TextEditingController();
  String _priority = 'normal';
  String? _category;
  String? _subscriptionId;
  bool _submitting = false;
  List<SubscriptionModel> _subscriptions = [];
  bool _loadingSubs = true;

  /// Pending attachments: (filePath, fileName, contentType)
  final List<_PendingAttachment> _pendingAttachments = [];

  @override
  void initState() {
    super.initState();
    _loadSubscriptions();
  }

  Future<void> _loadSubscriptions() async {
    try {
      final svc = ref.read(subscriptionServiceProvider);
      final result = await svc.list(perPage: 50);
      final paginated = result.getOrThrow();
      if (!mounted) return;
      setState(() {
        _subscriptions = paginated.data;
        _loadingSubs = false;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() => _loadingSubs = false);
    }
  }

  @override
  void dispose() {
    _subjectCtrl.dispose();
    _descriptionCtrl.dispose();
    super.dispose();
  }

  Future<void> _pickFile() async {
    // Show bottom sheet asking the user to pick a source — same UX as
    // profile avatar upload. Camera path uses image_picker which shows
    // the system CAMERA permission dialog automatically on first use.
    final isp = context.isp; // local — class doesn't have a persistent field
    final source = await showModalBottomSheet<_AttachmentSource>(
      context: context,
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              leading: Icon(Icons.photo_camera_outlined, color: isp.accent),
              title: const Text('Ambil Foto'),
              subtitle: const Text(
                'Kamera — perlu izin akses kamera',
                style: TextStyle(fontSize: 12),
              ),
              onTap: () => Navigator.pop(ctx, _AttachmentSource.camera),
            ),
            ListTile(
              leading: Icon(Icons.folder_open_outlined, color: isp.accent),
              title: const Text('Pilih File'),
              subtitle: const Text(
                'PDF, gambar, dokumen — dari penyimpanan perangkat',
                style: TextStyle(fontSize: 12),
              ),
              onTap: () => Navigator.pop(ctx, _AttachmentSource.files),
            ),
            const SizedBox(height: 8),
          ],
        ),
      ),
    );
    if (source == null) return;

    if (source == _AttachmentSource.camera) {
      await _captureFromCamera();
    } else {
      await _pickFromFiles();
    }
  }

  /// Capture a single photo via system camera.
  /// image_picker handles the CAMERA permission request internally and shows
  /// the system permission dialog on first use.
  Future<void> _captureFromCamera() async {
    final isp = context.isp; // local — class doesn't have a persistent field
    try {
      final picker = ImagePicker();
      final picked = await picker.pickImage(
        source: ImageSource.camera,
        maxWidth: 1600,
        maxHeight: 1600,
        imageQuality: 90,
      );
      if (picked == null || !mounted) return;

      final size = await picked.length();
      if (!mounted) return;

      setState(() {
        // Avoid duplicates if user re-picks same file
        if (!_pendingAttachments.any((a) => a.filePath == picked.path)) {
          _pendingAttachments.add(
            _PendingAttachment(
              filePath: picked.path,
              fileName: picked.name,
              contentType: 'image/jpeg',
              size: size,
            ),
          );
        }
      });
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Gagal membuka kamera: $e'),
          backgroundColor: isp.danger,
        ),
      );
    }
  }

  /// Pick one or more files via system file explorer (SAF — no permission needed).
  Future<void> _pickFromFiles() async {
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.any,
        allowMultiple: true,
      );
      if (result == null || result.files.isEmpty) return;

      setState(() {
        for (final file in result.files) {
          if (file.path == null) continue;
          if (_pendingAttachments.any((a) => a.filePath == file.path)) continue;
          _pendingAttachments.add(
            _PendingAttachment(
              filePath: file.path!,
              fileName: file.name,
              contentType: _guessContentType(file.name),
              size: file.size,
            ),
          );
        }
      });
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Gagal memilih file: $e')),
      );
    }
  }

  void _removeAttachment(int index) {
    setState(() => _pendingAttachments.removeAt(index));
  }

  Future<List<String>> _uploadPendingAttachments(String ticketId) async {
    if (_pendingAttachments.isEmpty) return [];

    final storageSvc = ref.read(storageServiceProvider);
    final ids = <String>[];

    for (final att in _pendingAttachments) {
      final ServiceResult<String> res = await storageSvc.uploadFile(
        filePath: att.filePath,
        fileName: att.fileName,
        contentType: att.contentType,
        ticketId: ticketId,
        supportTicketAttachment: true,
      );
      switch (res) {
        case Success(:final data):
          ids.add(data);
        case Failure(:final exception):
          throw exception.message;
      }
    }

    return ids;
  }

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;
    setState(() => _submitting = true);

    try {
      // Create the ticket first. Attachment upload needs a ticket_id for customer authorization.
      final svc = ref.read(ticketServiceProvider);
      final ServiceResult<TicketModel> res = await svc.create(
        subject: _subjectCtrl.text.trim(),
        message: _descriptionCtrl.text.trim(),
        priority: _priority,
        category: _category,
        subscriptionId: _subscriptionId,
      );

      if (!mounted) return;
      switch (res) {
        case Success(:final data):
          final ticket = data;
          if (_pendingAttachments.isNotEmpty) {
            final attachmentIds = await _uploadPendingAttachments(ticket.id);
            if (attachmentIds.isNotEmpty) {
              await svc.reply(
                ticketId: ticket.id,
                message: '(Lampiran)',
                attachmentIds: attachmentIds,
              );
            }
          }
          if (!mounted) return;
          context.go('/tickets/${ticket.id}');
        case Failure(:final exception):
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(exception.message)),
          );
      }
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Gagal mengirim: $e')),
      );
    } finally {
      if (mounted) setState(() => _submitting = false);
    }
  }

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.newTicket)),
      body: Form(
        key: _formKey,
        child: ListView(
          padding: const EdgeInsets.all(IspSpacing.lg),
          children: [
            // ── Subject ──
            IspCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.title,
                          size: 18, color: isp.accent),
                      const SizedBox(width: IspSpacing.sm),
                      Text(
                        'Subjek',
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                          color: isp.textSecondary,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: IspSpacing.md),
                  TextFormField(
                    controller: _subjectCtrl,
                    decoration: const InputDecoration(
                      hintText: 'Ringkasan masalah Anda',
                    ),
                    validator: (v) => (v == null || v.trim().length < 3)
                        ? 'Subjek minimal 3 karakter'
                        : null,
                  ),
                ],
              ),
            ),
            const SizedBox(height: IspSpacing.md),

            // ── Description ──
            IspCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.description_outlined,
                          size: 18, color: isp.accent),
                      const SizedBox(width: IspSpacing.sm),
                      Text(
                        'Deskripsi Masalah',
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                          color: isp.textSecondary,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: IspSpacing.md),
                  TextFormField(
                    controller: _descriptionCtrl,
                    maxLines: 6,
                    decoration: const InputDecoration(
                      hintText: 'Jelaskan masalah Anda secara detail...',
                      alignLabelWithHint: true,
                    ),
                    validator: (v) => (v == null || v.trim().length < 10)
                        ? 'Deskripsi minimal 10 karakter'
                        : null,
                  ),
                ],
              ),
            ),
            const SizedBox(height: IspSpacing.md),

            // ── Priority ──
            IspCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.flag_outlined,
                          size: 18, color: isp.accent),
                      const SizedBox(width: IspSpacing.sm),
                      Text(
                        'Prioritas',
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                          color: isp.textSecondary,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: IspSpacing.md),
                  DropdownButtonFormField<String>(
                    value: _priority,
                    isExpanded: true,
                    decoration: const InputDecoration(
                      contentPadding:
                          EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                    ),
                    items: const [
                      DropdownMenuItem(value: 'low', child: Text('Rendah')),
                      DropdownMenuItem(value: 'normal', child: Text('Normal')),
                      DropdownMenuItem(value: 'high', child: Text('Tinggi')),
                      DropdownMenuItem(
                          value: 'urgent', child: Text('Mendesak')),
                    ],
                    onChanged: (v) {
                      if (v != null) setState(() => _priority = v);
                    },
                  ),
                ],
              ),
            ),
            const SizedBox(height: IspSpacing.md),

            // ── Category ──
            IspCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.category_outlined,
                          size: 18, color: isp.accent),
                      const SizedBox(width: IspSpacing.sm),
                      Text(
                        'Kategori',
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                          color: isp.textSecondary,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: IspSpacing.md),
                  DropdownButtonFormField<String>(
                    value: _category,
                    isExpanded: true,
                    hint: const Text('Pilih kategori (opsional)'),
                    decoration: const InputDecoration(
                      contentPadding:
                          EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                    ),
                    items: const [
                      DropdownMenuItem(
                          value: null, child: Text('Tidak terkait')),
                      DropdownMenuItem(value: 'general', child: Text('Umum')),
                      DropdownMenuItem(
                          value: 'billing', child: Text('Tagihan')),
                      DropdownMenuItem(
                          value: 'technical', child: Text('Teknis')),
                      DropdownMenuItem(
                          value: 'installation', child: Text('Instalasi')),
                    ],
                    onChanged: (v) => setState(() => _category = v),
                  ),
                ],
              ),
            ),
            const SizedBox(height: IspSpacing.md),

            // ── Subscription ──
            IspCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.wifi_outlined,
                          size: 18, color: isp.accent),
                      const SizedBox(width: IspSpacing.sm),
                      Text(
                        'Langganan Terkait',
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                          color: isp.textSecondary,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: IspSpacing.md),
                  if (_loadingSubs)
                    const Padding(
                      padding: EdgeInsets.all(8),
                      child: SizedBox(
                        width: 20,
                        height: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                    )
                  else
                    DropdownButtonFormField<String>(
                      value: _subscriptionId,
                      isExpanded: true,
                      hint: const Text('Tidak terkait langganan'),
                      decoration: const InputDecoration(
                        contentPadding:
                            EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                      ),
                      items: [
                        const DropdownMenuItem(
                            value: null, child: Text('Tidak terkait')),
                        ..._subscriptions.map(
                          (s) => DropdownMenuItem(
                            value: s.id,
                            child: Text(
                              s.packageName ?? 'Langganan',
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ),
                      ],
                      onChanged: (v) => setState(() => _subscriptionId = v),
                    ),
                ],
              ),
            ),
            const SizedBox(height: IspSpacing.md),

            // ── Attachments ──
            IspCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(Icons.attach_file,
                          size: 18, color: isp.accent),
                      const SizedBox(width: IspSpacing.sm),
                      Text(
                        'Lampiran',
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                          color: isp.textSecondary,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: IspSpacing.md),
                  OutlinedButton.icon(
                    onPressed: _submitting ? null : _pickFile,
                    icon: const Icon(Icons.add),
                    label: const Text('Tambah File'),
                  ),
                  if (_pendingAttachments.isNotEmpty) ...[
                    const SizedBox(height: IspSpacing.sm),
                    Wrap(
                      spacing: IspSpacing.sm,
                      runSpacing: IspSpacing.xs,
                      children: List.generate(_pendingAttachments.length, (i) {
                        final att = _pendingAttachments[i];
                        return Chip(
                          avatar: Icon(
                            att.isImage ? Icons.image : Icons.attach_file,
                            size: 18,
                          ),
                          label: Text(
                            att.fileName,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          deleteIcon: const Icon(Icons.close, size: 18),
                          onDeleted: () => _removeAttachment(i),
                        );
                      }),
                    ),
                  ],
                ],
              ),
            ),
            const SizedBox(height: IspSpacing.xl),

            // ── Submit button ──
            ElevatedButton.icon(
              onPressed: _submitting ? null : _submit,
              icon: _submitting
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.send),
              label: const Text('Kirim'),
            ),
          ],
        ),
      ),
    );
  }
}

class _PendingAttachment {
  const _PendingAttachment({
    required this.filePath,
    required this.fileName,
    required this.contentType,
    required this.size,
  });
  final String filePath;
  final String fileName;
  final String contentType;
  final int size;

  bool get isImage => contentType.startsWith('image/');
}

String _guessContentType(String fileName) {
  final ext = fileName.split('.').last.toLowerCase();
  switch (ext) {
    case 'jpg':
    case 'jpeg':
      return 'image/jpeg';
    case 'png':
      return 'image/png';
    case 'gif':
      return 'image/gif';
    case 'webp':
      return 'image/webp';
    case 'pdf':
      return 'application/pdf';
    case 'doc':
    case 'docx':
      return 'application/msword';
    case 'xls':
    case 'xlsx':
      return 'application/vnd.ms-excel';
    case 'zip':
      return 'application/zip';
    case 'rar':
      return 'application/x-rar-compressed';
    case 'txt':
      return 'text/plain';
    default:
      return 'application/octet-stream';
  }
}
