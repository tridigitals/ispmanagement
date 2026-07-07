import 'package:api_client/api_client.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:image_picker/image_picker.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/service_providers.dart';
import '../../services/missing_providers.dart';

/// Attachment picker source — mirrors the profile-upload pattern so the
/// camera permission flow stays consistent across the app.
enum _AttachmentSource { camera, files }

/// One-tap shortcut for the most common outage types. Auto-fills subject,
/// description AND category so admins can triage without re-classifying.
class _QuickAction {
  const _QuickAction({
    required this.icon,
    required this.labelKey,
    required this.subjectKey,
    required this.descriptionKey,
    required this.category,
  });
  final IconData icon;
  final String labelKey; // i18n key suffix under ticketQuickAction*
  final String subjectKey;
  final String descriptionKey;
  final String category; // 'general' | 'technical' | etc.
}

List<_QuickAction> _buildQuickActions(AppLocalizations l10n) => [
      _QuickAction(
        icon: Icons.wifi_off_rounded,
        labelKey: 'NoInternet',
        subjectKey: 'NoInternetSubject',
        descriptionKey: 'NoInternetDesc',
        category: 'technical',
      ),
      _QuickAction(
        icon: Icons.network_check_rounded,
        labelKey: 'Slow',
        subjectKey: 'SlowSubject',
        descriptionKey: 'SlowDesc',
        category: 'technical',
      ),
      _QuickAction(
        icon: Icons.edit_note_rounded,
        labelKey: 'Other',
        subjectKey: 'Other',
        descriptionKey: 'Other',
        category: 'general',
      ),
    ];

String _quickLabel(AppLocalizations l, _QuickAction a) {
  switch (a.labelKey) {
    case 'NoInternet':
      return l.ticketQuickActionNoInternet;
    case 'Slow':
      return l.ticketQuickActionSlow;
    case 'Other':
      return l.ticketQuickActionOther;
  }
  return '';
}

String _quickSubject(AppLocalizations l, _QuickAction a) {
  switch (a.subjectKey) {
    case 'NoInternetSubject':
      return l.ticketQuickActionNoInternetSubject;
    case 'SlowSubject':
      return l.ticketQuickActionSlowSubject;
  }
  return '';
}

String _quickDesc(AppLocalizations l, _QuickAction a) {
  switch (a.descriptionKey) {
    case 'NoInternetDesc':
      return l.ticketQuickActionNoInternetDesc;
    case 'SlowDesc':
      return l.ticketQuickActionSlowDesc;
  }
  return '';
}

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

  /// Category set by quick-action chip (or null if user picks "Other"
  /// then clears it). Backend uses this for routing/admin triage.
  String? _selectedCategory;

  /// Optional subscription link. null = not linked. Picked from user's
  /// active subscriptions in a bottom sheet.
  String? _selectedSubscriptionId;

  /// Pending attachments: (filePath, fileName, contentType, size)
  final List<_PendingAttachment> _pendingAttachments = [];

  @override
  void dispose() {
    _subjectCtrl.dispose();
    _descriptionCtrl.dispose();
    super.dispose();
  }

  void _applyQuickAction(_QuickAction action, AppLocalizations l) {
    _subjectCtrl.text = _quickSubject(l, action);
    _descriptionCtrl.text = _quickDesc(l, action);
    setState(() => _selectedCategory = action.category);
  }

  // ── Attachment picker (bottom sheet → camera OR file picker) ──

  Future<void> _pickFile() async {
    final isp = context.isp; // local — class has no persistent field
    final l10n = AppLocalizations.of(context);
    final source = await showModalBottomSheet<_AttachmentSource>(
      context: context,
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              leading: Icon(Icons.photo_camera_outlined, color: isp.accent),
              title: Text(l10n.ticketActionCamera),
              subtitle: Text(
                l10n.ticketActionCameraSub,
                style: const TextStyle(fontSize: 12),
              ),
              onTap: () => Navigator.pop(ctx, _AttachmentSource.camera),
            ),
            ListTile(
              leading: Icon(Icons.folder_open_outlined, color: isp.accent),
              title: Text(l10n.ticketActionFile),
              subtitle: Text(
                l10n.ticketActionFileSub,
                style: const TextStyle(fontSize: 12),
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
    final l10n = AppLocalizations.of(context);
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
          content: Text(l10n.ticketErrorCameraFailed(e.toString())),
          backgroundColor: isp.danger,
        ),
      );
    }
  }

  Future<void> _pickFromFiles() async {
    final l10n = AppLocalizations.of(context);
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
        SnackBar(content: Text(l10n.ticketErrorFileFailed(e.toString()))),
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
    final l10n = AppLocalizations.of(context);
    final messenger = ScaffoldMessenger.of(context);
    final router = GoRouter.of(context);

    try {
      // 1. Create ticket — pass category (from quick-action) and
      //    subscriptionId (if user picked one) so admins get full context
      //    immediately instead of having to re-classify.
      final svc = ref.read(ticketServiceProvider);
      final ServiceResult<TicketModel> res = await svc.create(
        subject: _subjectCtrl.text.trim(),
        message: _descriptionCtrl.text.trim(),
        category: _selectedCategory,
        subscriptionId: _selectedSubscriptionId,
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
              content: Text(l10n.ticketToastCreated),
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
          content: Text(l10n.ticketErrorSendFailed(e.toString())),
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
    final quickActions = _buildQuickActions(l10n);

    return Scaffold(
      // Keyboard overlays the body instead of squeezing it. The sticky
      // submit button uses viewInsets.bottom padding so it floats above
      // the keyboard regardless.
      resizeToAvoidBottomInset: false,
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
                        for (final action in quickActions)
                          ActionChip(
                            avatar: Icon(action.icon, size: 18, color: isp.accent),
                            label: Text(_quickLabel(l10n, action)),
                            onPressed: () => _applyQuickAction(action, l10n),
                          ),
                      ],
                    ),
                    const SizedBox(height: IspSpacing.md),

                    // ── Single card: subject + description + attachments + subscription ──
                    IspCard(
                      nbStyle: true,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          // Subject
                          TextFormField(
                            controller: _subjectCtrl,
                            autofocus: true,
                            textCapitalization: TextCapitalization.sentences,
                            decoration: InputDecoration(
                              labelText: l10n.ticketFieldSubject,
                              hintText: l10n.ticketFieldSubjectHint,
                            ),
                            validator: (v) =>
                                (v == null || v.trim().length < 3)
                                    ? l10n.ticketValidationSubjectShort
                                    : null,
                          ),
                          const SizedBox(height: IspSpacing.md),

                          // Description
                          TextFormField(
                            controller: _descriptionCtrl,
                            minLines: 4,
                            maxLines: 8,
                            textCapitalization: TextCapitalization.sentences,
                            decoration: InputDecoration(
                              labelText: l10n.ticketFieldDescription,
                              hintText: l10n.ticketFieldDescriptionHint,
                              alignLabelWithHint: true,
                            ),
                            validator: (v) =>
                                (v == null || v.trim().length < 10)
                                    ? l10n.ticketValidationDescriptionShort
                                    : null,
                          ),
                          const SizedBox(height: IspSpacing.md),

                          // Subscription picker (optional)
                          _SubscriptionPicker(
                            selectedId: _selectedSubscriptionId,
                            onChanged: (id) => setState(() {
                              _selectedSubscriptionId = id;
                            }),
                          ),
                          const SizedBox(height: IspSpacing.lg),

                          // Attachments
                          Row(
                            children: [
                              Icon(
                                Icons.attach_file,
                                size: 18,
                                boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
                                          border: Border.all(color: isp.border, width: 1.5),
                                                    color: isp.surface,
                                                              border: Border.all(color: isp.border, width: 1.5),
                                                              boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
                                                              borderRadius:
                              const SizedBox(width: IspSpacing.sm),
                              Text(
                                l10n.ticketFieldAttachments,
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
                                      decoration: BoxDecoration(
                                        color: isp.accent.withOpacity(0.1),
                                        borderRadius: BorderRadius.circular(10),
                                        border: Border.all(color: isp.border, width: 1.5),
                                        boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
                                      ),
                                ),
                              ],
                              const Spacer(),
                              TextButton.icon(
                                onPressed:
                                    _submitting ? null : _pickFile,
                                icon: const Icon(Icons.add, size: 18),
                                label: Text(l10n.ticketButtonAdd),
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
                            border: Border.all(color: isp.border, width: 1.5),
                            boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
                            borderRadius:
                    top: BorderSide(color: isp.border),
                  ),
                  boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
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
                      _submitting ? l10n.ticketButtonSending : l10n.ticketButtonSend,
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

/// Subscription picker — opens a bottom sheet listing the user's active
/// subscriptions so the ticket can be linked to one. Optional — user can
/// leave it unlinked (default).
class _SubscriptionPicker extends ConsumerWidget {
  const _SubscriptionPicker({
    required this.selectedId,
    required this.onChanged,
  });

  final String? selectedId;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final isp = context.isp;
    final asyncSubs = ref.watch(mySubscriptionsProvider);

    String displayLabel = l10n.ticketFieldNoSubscription;
    if (selectedId != null && asyncSubs.value != null) {
      final match = asyncSubs.value!.where((s) => s.id == selectedId).firstOrNull;
      if (match != null) {
        displayLabel = '${match.packageName ?? l10n.internetPackage} • ${match.id.substring(0, 8)}';
      }
    }

    return InkWell(
      onTap: () async {
        final subs = asyncSubs.value ?? [];
        if (subs.isEmpty) return;
        final picked = await showModalBottomSheet<String?>(
          context: context,
          isScrollControlled: true,
          builder: (ctx) => SafeArea(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                ListTile(
                  leading: Icon(Icons.link_off, color: isp.textMuted),
                  title: Text(l10n.ticketFieldNoSubscription),
                  onTap: () => Navigator.pop(ctx, null),
                ),
                const Divider(height: 1),
                for (final s in subs)
                  ListTile(
                    leading: Icon(
                      selectedId == s.id ? Icons.check_circle : Icons.wifi,
                      color: selectedId == s.id ? isp.accent : isp.textMuted,
                    ),
                    title: Text(s.packageName ?? l10n.internetPackage),
                    subtitle: Text('#${s.id.substring(0, 8)}'),
                    onTap: () => Navigator.pop(ctx, s.id),
                  ),
                const SizedBox(height: 8),
              ],
            ),
          ),
        );
        if (picked != null || (selectedId != null)) {
          // distinguish "tapped none" (no change) from "explicitly cleared"
          onChanged(picked);
        }
      },
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
        decoration: BoxDecoration(
          border: Border.all(color: isp.border, width: 1.5),
          borderRadius: BorderRadius.circular(8),
          boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
        ),
        child: Row(
          children: [
            Icon(Icons.wifi, size: 18, color: isp.accent),
            const SizedBox(width: IspSpacing.sm),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    l10n.ticketFieldSubscription,
                    style: TextStyle(
                      fontSize: 12,
                      color: isp.textMuted,
                    ),
                  ),
                  Text(
                    displayLabel,
                    style: TextStyle(
                      fontSize: 14,
                      color: isp.textPrimary,
                      fontWeight: FontWeight.w500,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            Icon(Icons.chevron_right, color: isp.textMuted),
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
