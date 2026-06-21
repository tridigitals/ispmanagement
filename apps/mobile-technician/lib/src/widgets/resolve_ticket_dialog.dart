// Sprint 3: Ticket resolve dialog — notes + photo attachments + signature pad.
//
// Bottom-sheet style dialog shown from ticket detail when tech taps "Selesaikan".
// Workflow:
//   1. Tech enters completion notes
//   2. Takes photo(s) with camera (image_picker)
//   3. Uploads all photos on submit (multipart)
//   4. Captures signature via Signature pad
//   5. Tap "Selesaikan" → calls ticketActionController.resolve()
//   6. On success, dialog pops + snackbar + ticket detail refreshes.

import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:image_picker/image_picker.dart';
import 'package:signature/signature.dart';

import '../../services/ticket_actions.dart';

class ResolveTicketDialog extends ConsumerStatefulWidget {
  const ResolveTicketDialog({super.key, required this.ticketId});

  final String ticketId;

  @override
  ConsumerState<ResolveTicketDialog> createState() => _ResolveTicketDialogState();
}

class _ResolveTicketDialogState extends ConsumerState<ResolveTicketDialog> {
  final _notesController = TextEditingController();
  final _signatureController = SignatureController(
    penStrokeWidth: 3,
    penColor: Colors.black,
    exportBackgroundColor: Colors.white,
  );
  final _picker = ImagePicker();
  bool _submitting = false;

  @override
  void dispose() {
    _notesController.dispose();
    _signatureController.dispose();
    super.dispose();
  }

  Future<void> _capturePhoto(ImageSource source) async {
    try {
      final xfile = await _picker.pickImage(
        source: source,
        maxWidth: 1920,
        maxHeight: 1080,
        imageQuality: 85,
      );
      if (xfile == null) return;
      ref
          .read(resolveDraftProvider.notifier)
          .addPhoto(xfile.path);
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Gagal ambil foto: $e')),
      );
    }
  }

  void _showPhotoSourceSheet() {
    showModalBottomSheet<void>(
      context: context,
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              leading: const Icon(Icons.camera_alt),
              title: const Text('Kamera'),
              onTap: () {
                Navigator.of(ctx).pop();
                _capturePhoto(ImageSource.camera);
              },
            ),
            ListTile(
              leading: const Icon(Icons.photo_library),
              title: const Text('Galeri'),
              onTap: () {
                Navigator.of(ctx).pop();
                _capturePhoto(ImageSource.gallery);
              },
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _submit() async {
    final draft = ref.read(resolveDraftProvider);
    if (draft.notes.trim().isEmpty && !draft.hasPhotos && !draft.hasSignature) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Tambahkan catatan, foto, atau tanda tangan'),
        ),
      );
      return;
    }

    setState(() => _submitting = true);

    // Upload all photos first.
    await ref
        .read(resolveDraftProvider.notifier)
        .uploadAllPhotos(ref, widget.ticketId);

    // Capture signature bytes (if any).
    Uint8List? sigBytes;
    if (_signatureController.isNotEmpty) {
      sigBytes = await _signatureController.toPngBytes();
      if (sigBytes != null) {
        ref.read(resolveDraftProvider.notifier).setSignature(sigBytes);
      }
    }

    final updated = await ref.read(ticketActionControllerProvider).resolve(
          ticketId: widget.ticketId,
          draft: ref.read(resolveDraftProvider),
        );

    if (!mounted) return;
    setState(() => _submitting = false);

    if (updated != null) {
      ref.read(resolveDraftProvider.notifier).reset();
      Navigator.of(context).pop(true);
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Tiket berhasil diselesaikan')),
      );
    } else {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Gagal menyelesaikan tiket. Coba lagi.')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final draft = ref.watch(resolveDraftProvider);
    final theme = Theme.of(context);

    return Dialog(
      insetPadding: const EdgeInsets.all(12),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 600, maxHeight: 720),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 16, 20, 12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  Icon(Icons.check_circle_outline,
                      color: theme.colorScheme.primary),
                  const SizedBox(width: 8),
                  Text(
                    'Selesaikan Tiket',
                    style: theme.textTheme.titleLarge,
                  ),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: _submitting ? null : () => Navigator.of(context).pop(false),
                  ),
                ],
              ),
              const Divider(height: 16),

              Flexible(
                child: SingleChildScrollView(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      // Notes
                      TextField(
                        controller: _notesController,
                        minLines: 3,
                        maxLines: 6,
                        decoration: const InputDecoration(
                          labelText: 'Catatan Penyelesaian',
                          hintText:
                              'Jelaskan perbaikan / penggantian yang dilakukan...',
                          border: OutlineInputBorder(),
                        ),
                        onChanged: (v) =>
                            ref.read(resolveDraftProvider.notifier).setNotes(v),
                      ),
                      const SizedBox(height: 16),

                      // Photos
                      Row(
                        children: [
                          const Icon(Icons.photo_camera_outlined, size: 20),
                          const SizedBox(width: 6),
                          Text('Foto Bukti',
                              style: theme.textTheme.titleSmall),
                          const Spacer(),
                          TextButton.icon(
                            onPressed: _submitting ? null : _showPhotoSourceSheet,
                            icon: const Icon(Icons.add_a_photo, size: 18),
                            label: const Text('Tambah'),
                          ),
                        ],
                      ),
                      if (draft.photos.isEmpty)
                        Padding(
                          padding: const EdgeInsets.symmetric(vertical: 12),
                          child: Text(
                            'Belum ada foto. Tambahkan bukti perbaikan.',
                            style: TextStyle(
                              color: theme.hintColor,
                              fontStyle: FontStyle.italic,
                            ),
                          ),
                        )
                      else
                        SizedBox(
                          height: 110,
                          child: ListView.separated(
                            scrollDirection: Axis.horizontal,
                            itemCount: draft.photos.length,
                            separatorBuilder: (_, __) => const SizedBox(width: 8),
                            itemBuilder: (ctx, idx) {
                              final photo = draft.photos[idx];
                              return Stack(
                                children: [
                                  ClipRRect(
                                    borderRadius: BorderRadius.circular(8),
                                    child: SizedBox(
                                      width: 110,
                                      height: 110,
                                      child: photo.uploading
                                          ? const Center(
                                              child: SizedBox(
                                                width: 28,
                                                height: 28,
                                                child:
                                                    CircularProgressIndicator(
                                                        strokeWidth: 2),
                                              ),
                                            )
                                          : Image.file(
                                              File(photo.localPath),
                                              fit: BoxFit.cover,
                                              errorBuilder: (_, __, ___) =>
                                                  const ColoredBox(
                                                color: Colors.black12,
                                                child: Icon(Icons.broken_image),
                                              ),
                                            ),
                                    ),
                                  ),
                                  if (photo.isUploaded)
                                    Positioned(
                                      bottom: 4,
                                      right: 4,
                                      child: Container(
                                        decoration: BoxDecoration(
                                          color: Colors.green,
                                          borderRadius:
                                              BorderRadius.circular(10),
                                        ),
                                        padding: const EdgeInsets.all(3),
                                        child: const Icon(
                                          Icons.check,
                                          color: Colors.white,
                                          size: 14,
                                        ),
                                      ),
                                    ),
                                  Positioned(
                                    top: 2,
                                    right: 2,
                                    child: InkWell(
                                      onTap: _submitting
                                          ? null
                                          : () => ref
                                              .read(resolveDraftProvider.notifier)
                                              .removePhoto(idx),
                                      child: Container(
                                        decoration: BoxDecoration(
                                          color: Colors.black54,
                                          borderRadius:
                                              BorderRadius.circular(12),
                                        ),
                                        padding: const EdgeInsets.all(2),
                                        child: const Icon(
                                          Icons.close,
                                          color: Colors.white,
                                          size: 16,
                                        ),
                                      ),
                                    ),
                                  ),
                                ],
                              );
                            },
                          ),
                        ),
                      const SizedBox(height: 16),

                      // Signature
                      Row(
                        children: [
                          const Icon(Icons.draw_outlined, size: 20),
                          const SizedBox(width: 6),
                          Text('Tanda Tangan Teknisi',
                              style: theme.textTheme.titleSmall),
                          const Spacer(),
                          TextButton.icon(
                            onPressed: _submitting
                                ? null
                                : () => _signatureController.clear(),
                            icon: const Icon(Icons.refresh, size: 18),
                            label: const Text('Reset'),
                          ),
                        ],
                      ),
                      Container(
                        height: 140,
                        decoration: BoxDecoration(
                          color: Colors.grey.shade50,
                          border: Border.all(color: Colors.grey.shade300),
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: ClipRRect(
                          borderRadius: BorderRadius.circular(8),
                          child: Signature(
                            controller: _signatureController,
                            backgroundColor: Colors.transparent,
                          ),
                        ),
                      ),
                      const SizedBox(height: 8),
                      Text(
                        'Tanda tangan wajib untuk tiket instalasi / perbaikan.',
                        style: TextStyle(
                          color: theme.hintColor,
                          fontSize: 12,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 12),
              SizedBox(
                height: 48,
                child: FilledButton.icon(
                  onPressed: _submitting ? null : _submit,
                  icon: _submitting
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(
                              strokeWidth: 2, color: Colors.white),
                        )
                      : const Icon(Icons.check),
                  label: Text(_submitting ? 'Mengirim...' : 'Selesaikan'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}