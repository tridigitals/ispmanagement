import 'package:api_client/api_client.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:image_picker/image_picker.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';

/// Attachment picker source — mirrors the profile-upload pattern so the
/// camera permission flow stays consistent across the app.
enum _AttachmentSource { camera, files }

/// One-tap shortcut for the most common outage types. Auto-fills subject +
/// description so the user only edits if they want to add detail.
class _QuickAction {
  const _QuickAction({
    required this.icon,
    required this.label,
    required this.subject,
    required this.description,
  });
  final IconData icon;
  final String label;
  final String subject;
  final String description;
}

const _quickActions = <_QuickAction>[
  _QuickAction(
    icon: Icons.wifi_off_rounded,
    label: 'Internet Mati',
    subject: 'Internet tidak bisa diakses',
    description:
        'Koneksi internet di lokasi saya tidak dapat diakses. Mohon dicek.',
  ),
  _QuickAction(
    icon: Icons.network_check_rounded,
    label: 'WiFi Lemot',
    subject: 'WiFi lambat / sering putus',
    description:
        'Koneksi WiFi terasa lambat atau tidak stabil. Mohon dicek.',
  ),
  _QuickAction(
    icon: Icons.edit_note_rounded,
    label: 'Lainnya',
    subject: '',
    description: '',
  ),
];

class NewTicketScreen extends ConsumerStatefulWidget {
  const NewTicketScreen({super.key});

  @override
  ConsumerState<NewTicketScreen> createState() => _NewTicketScreenState();
}

class _NewTicketScreenState extends ConsumerState<NewTicketScreen> {
  final _formKey = GlobalKey<FormState>();
  final _subjectCtrl = TextEditingController();
  final _descriptionCtrl = TextEditingController();
  bool _submitting = false;

  /// Pending attachments: (filePath, fileName, contentType, size)
  final List<_PendingAttachment> _pendingAttachments = [];

  @override
  void dispose() {
    _subjectCtrl.dispose();
    _descriptionCtrl.dispose();
    super.dispose();
  }

  void _applyQuickAction(_QuickAction action) {
    _subjectCtrl.text = action.subject;
    _descriptionCtrl.text = action.description;
  }

  // ── Attachment picker (bottom sheet → camera OR file picker) ──

  Future<void> _pickFile() async {
    final isp = context.isp; // local — class has no persistent field
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

  Future<void> _captureFromCamera() async {
    final isp = context.isp;
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

  // ── Upload + submit ──

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

    final isp = context.isp;
    final messenger = ScaffoldMessenger.of(context);
    final router = GoRouter.of(context);

    try {
      // 1. Create ticket (priority + category are admin-triaged; backend
      //    defaults priority to 'normal' if not provided).
      final svc = ref.read(ticketServiceProvider);
      final ServiceResult<TicketModel> res = await svc.create(
        subject: _subjectCtrl.text.trim(),
        message: _descriptionCtrl.text.trim(),
      );

      if (!mounted) return;
      switch (res) {
        case Success(:final data):
          final ticket = data;
          // 2. Upload attachments if any (need ticket_id for auth).
          if (_pendingAttachments.isNotEmpty) {
            final attachmentIds = await _uploadPendingAttachments(ticket.id);
            if (attachmentIds.isNotEmpty) {
              // Backend requires non-empty message on reply, so we send a
              // placeholder so the attachment lands in the ticket thread.
              await svc.reply(
                ticketId: ticket.id,
                message: '(Lampiran)',
                attachmentIds: attachmentIds,
              );
            }
          }
          if (!mounted) return;
          messenger.showSnackBar(
            SnackBar(
              content: const Text('Tiket terkirim — tim kami akan menindak lanjuti'),
              backgroundColor: isp.success,
            ),
          );
          // Pop back to wherever we came from (support tab or subscription
          // detail) — don't push to detail, user can tap the new ticket if
          // they want to see it.
          if (router.canPop()) {
            router.pop();
          } else {
            router.go('/tickets');
          }
        case Failure(:final exception):
          messenger.showSnackBar(
            SnackBar(
              content: Text(exception.message),
              backgroundColor: isp.danger,
            ),
          );
      }
    } catch (e) {
      if (!mounted) return;
      messenger.showSnackBar(
        SnackBar(
          content: Text('Gagal mengirim: $e'),
          backgroundColor: isp.danger,
        ),
      );
    } finally {
      if (mounted) setState(() => _submitting = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Scaffold(
      appBar: AppBar(title: Text(l10n.newTicket)),
      body: SafeArea(
        child: Form(
          key: _formKey,
          child: Column(
            children: [
              Expanded(
                child: ListView(
                  padding: const EdgeInsets.all(IspSpacing.lg),
                  children: [
                    // ── Quick action chips (one-tap shortcuts) ──
                    Wrap(
                      spacing: IspSpacing.sm,
                      runSpacing: IspSpacing.sm,
                      children: [
                        for (final action in _quickActions)
                          ActionChip(
                            avatar: Icon(action.icon, size: 18, color: isp.accent),
                            label: Text(action.label),
                            onPressed: () => _applyQuickAction(action),
                          ),
                      ],
                    ),
                    const SizedBox(height: IspSpacing.md),

                    // ── Single card: subject + description + attachments ──
                    IspCard(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          // Subject
                          TextFormField(
                            controller: _subjectCtrl,
                            autofocus: true,
                            textCapitalization: TextCapitalization.sentences,
                            decoration: const InputDecoration(
                              labelText: 'Subjek',
                              hintText: 'Ringkasan masalah',
                            ),
                            validator: (v) =>
                                (v == null || v.trim().length < 3)
                                    ? 'Subjek minimal 3 karakter'
                                    : null,
                          ),
                          const SizedBox(height: IspSpacing.md),

                          // Description
                          TextFormField(
                            controller: _descriptionCtrl,
                            minLines: 4,
                            maxLines: 8,
                            textCapitalization: TextCapitalization.sentences,
                            decoration: const InputDecoration(
                              labelText: 'Deskripsi',
                              hintText: 'Jelaskan masalah Anda...',
                              alignLabelWithHint: true,
                            ),
                            validator: (v) =>
                                (v == null || v.trim().length < 10)
                                    ? 'Deskripsi minimal 10 karakter'
                                    : null,
                          ),
                          const SizedBox(height: IspSpacing.lg),

                          // Attachments
                          Row(
                            children: [
                              Icon(
                                Icons.attach_file,
                                size: 18,
                                color: isp.accent,
                              ),
                              const SizedBox(width: IspSpacing.sm),
                              Text(
                                'Lampiran',
                                style: TextStyle(
                                  fontSize: 14,
                                  fontWeight: FontWeight.w600,
                                  color: isp.textSecondary,
                                ),
                              ),
                              if (_pendingAttachments.isNotEmpty) ...[
                                const SizedBox(width: IspSpacing.sm),
                                Container(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 8,
                                    vertical: 2,
                                  ),
                                  decoration: BoxDecoration(
                                    color: isp.accent.withOpacity(0.1),
                                    borderRadius: BorderRadius.circular(10),
                                  ),
                                  child: Text(
                                    '${_pendingAttachments.length}',
                                    style: TextStyle(
                                      fontSize: 12,
                                      fontWeight: FontWeight.w600,
                                      color: isp.accent,
                                    ),
                                  ),
                                ),
                              ],
                              const Spacer(),
                              TextButton.icon(
                                onPressed:
                                    _submitting ? null : _pickFile,
                                icon: const Icon(Icons.add, size: 18),
                                label: const Text('Tambah'),
                              ),
                            ],
                          ),
                          if (_pendingAttachments.isNotEmpty) ...[
                            const SizedBox(height: IspSpacing.sm),
                            Wrap(
                              spacing: IspSpacing.sm,
                              runSpacing: IspSpacing.xs,
                              children: List.generate(
                                _pendingAttachments.length,
                                (i) {
                                  final att = _pendingAttachments[i];
                                  return Chip(
                                    avatar: Icon(
                                      att.isImage
                                          ? Icons.image
                                          : Icons.attach_file,
                                      size: 18,
                                    ),
                                    label: Text(
                                      att.fileName,
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                    deleteIcon:
                                        const Icon(Icons.close, size: 18),
                                    onDeleted: () => _removeAttachment(i),
                                  );
                                },
                              ),
                            ),
                          ],
                        ],
                      ),
                    ),
                  ],
                ),
              ),

              // ── Sticky submit button (always visible above keyboard) ──
              Container(
                padding: EdgeInsets.fromLTRB(
                  IspSpacing.lg,
                  IspSpacing.sm,
                  IspSpacing.lg,
                  IspSpacing.sm + MediaQuery.viewInsetsOf(context).bottom,
                ),
                decoration: BoxDecoration(
                  color: isp.surface,
                  border: Border(
                    top: BorderSide(color: isp.border),
                  ),
                ),
                child: SizedBox(
                  width: double.infinity,
                  child: ElevatedButton.icon(
                    onPressed: _submitting ? null : _submit,
                    icon: _submitting
                        ? const SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.send),
                    label: Text(
                      _submitting ? 'Mengirim...' : 'Kirim Tiket',
                    ),
                  ),
                ),
              ),
            ],
          ),
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
