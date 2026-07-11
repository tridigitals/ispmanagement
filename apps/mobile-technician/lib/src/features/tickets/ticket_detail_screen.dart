import 'dart:async';
import 'dart:io';

import 'package:api_client/api_client.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:dio/dio.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_cache_manager/flutter_cache_manager.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:image_picker/image_picker.dart';
import 'package:intl/intl.dart';
import 'package:path_provider/path_provider.dart';
import 'package:share_plus/share_plus.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/app_config.dart';
import '../../services/auth_providers.dart';
import '../../services/service_providers.dart';
import 'ticket_l10n.dart';
// import 'ticket_satisfaction_survey.dart';  // disabled - not shown in technician app

/// Source for ticket attachments — mirrors the profile-upload pattern so
/// camera permission flow is consistent across the app.
enum _AttachmentSource { camera, files }

/// Current user's ID for staff detection.
final _currentUserIdProvider = Provider<String?>((ref) {
  return ref.watch(currentUserProvider)?.id;
});

/// Auth token for file access.
final _authTokenProvider = FutureProvider<String?>((ref) async {
  return ref.read(tokenStorageProvider).readToken();
});

final ticketByIdProvider =
    FutureProvider.family<TicketModel, String>((ref, id) async {
  final svc = ref.watch(ticketServiceProvider);
  final ServiceResult<TicketModel> res = await svc.getById(id);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw exception.message,
  };
});

final ticketMessagesProvider =
    FutureProvider.family<List<TicketMessageModel>, String>((ref, id) async {
  final svc = ref.watch(ticketServiceProvider);
  final userId = ref.watch(_currentUserIdProvider);
  final ServiceResult<List<TicketMessageModel>> res =
      await svc.listMessages(id, currentUserId: userId);
  return switch (res) {
    Success(:final data) => data,
    Failure(:final exception) => throw exception.message,
  };
});

class TicketDetailScreen extends ConsumerStatefulWidget {
  const TicketDetailScreen({required this.id, super.key});
  final String id;

  @override
  ConsumerState<TicketDetailScreen> createState() => _TicketDetailScreenState();
}

class _TicketDetailScreenState extends ConsumerState<TicketDetailScreen>
    with WidgetsBindingObserver {
  final _messageCtrl = TextEditingController();
  final _scrollCtrl = ScrollController();
  late final IspThemeColors isp;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }
  bool _sending = false;
  bool _uploading = false;
  Timer? _autoRefreshTimer;
  StreamSubscription<Map<String, dynamic>>? _realtimeSub;

  /// Pending attachments: list of (filePath, fileName, contentType).
  final List<_PendingAttachment> _pendingAttachments = [];

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    // Force fresh fetch when entering this screen (providers cache by ID)
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.invalidate(ticketByIdProvider(widget.id));
      ref.invalidate(ticketMessagesProvider(widget.id));
    });
    _autoRefreshTimer = Timer.periodic(
      const Duration(seconds: 30),
      (_) => _silentRefresh(),
    );
    // Listen to realtime WebSocket for instant message updates
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _subscribeRealtime();
    });
  }

  void _subscribeRealtime() {
    final client = ref.read(realtimeClientProvider);
    _realtimeSub = client.stream.listen((event) {
      final type = event['type'] as String?;
      if (type != 'support_ticket_message_created') return;
      // Fields are at top level (serde tagged enum), not in 'data' wrapper
      final ticketId = event['ticket_id'] as String?;
      if (ticketId == widget.id) {
        _silentRefresh();
        // Auto-scroll to bottom after new message loads
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (_scrollCtrl.hasClients) {
            _scrollCtrl.animateTo(
              _scrollCtrl.position.maxScrollExtent,
              duration: const Duration(milliseconds: 300),
              curve: Curves.easeOut,
            );
          }
        });
      }
    });
  }

  @override
  void dispose() {
    _realtimeSub?.cancel();
    _autoRefreshTimer?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    _messageCtrl.dispose();
    _scrollCtrl.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) _silentRefresh();
  }

  void _silentRefresh() {
    ref.invalidate(ticketMessagesProvider(widget.id));
  }

  Future<void> _pickAttachment() async {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    // Show bottom sheet asking the user to pick a source — same UX as
    // profile avatar upload. Camera path uses image_picker which shows
    // the system CAMERA permission dialog automatically on first use.
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

  /// Capture a single photo via system camera.
  /// image_picker handles the CAMERA permission request internally and shows
  /// the system permission dialog on first use.
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

  /// Pick one or more files via system file explorer (SAF — no permission needed).
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
          // Avoid duplicates
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

  Future<List<String>> _uploadPendingAttachments() async {
    if (_pendingAttachments.isEmpty) return [];
    setState(() => _uploading = true);

    final storageSvc = ref.read(storageServiceProvider);
    final ids = <String>[];

    try {
      for (final att in _pendingAttachments) {
        final ServiceResult<String> res = await storageSvc.uploadFile(
          filePath: att.filePath,
          fileName: att.fileName,
          contentType: att.contentType,
          ticketId: widget.id,
          supportTicketAttachment: true,
        );
        switch (res) {
          case Success(:final data):
            ids.add(data);
          case Failure(:final exception):
            throw exception.message;
        }
      }
    } finally {
      if (mounted) setState(() => _uploading = false);
    }

    return ids;
  }

  Future<void> _send() async {
    final text = _messageCtrl.text.trim();
    if (text.isEmpty && _pendingAttachments.isEmpty) return;

    setState(() => _sending = true);

    try {
      // Upload attachments first if any
      List<String> attachmentIds = [];
      if (_pendingAttachments.isNotEmpty) {
        attachmentIds = await _uploadPendingAttachments();
      }

      final res = await ref.read(ticketServiceProvider).reply(
            ticketId: widget.id,
            message: text.isEmpty ? '(Lampiran)' : text,
            attachmentIds: attachmentIds.isEmpty ? null : attachmentIds,
          );

      if (!mounted) return;
      switch (res) {
        case Success():
          _messageCtrl.clear();
          setState(() => _pendingAttachments.clear());
          ref.invalidate(ticketMessagesProvider(widget.id));
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (_scrollCtrl.hasClients) {
              _scrollCtrl.animateTo(
                _scrollCtrl.position.maxScrollExtent,
                duration: const Duration(milliseconds: 300),
                curve: Curves.easeOut,
              );
            }
          });
        case Failure(:final exception):
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(exception.message)),
          );
      }
    } catch (e) {
      if (!mounted) return;
      final l10n = AppLocalizations.of(context);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.ticketErrorReplyFailed(e.toString()))),
      );
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  void _showSubscriptionSheet(BuildContext context, String subscriptionId) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) => _SubscriptionInfoSheet(subscriptionId: subscriptionId),
    );
  }

  Future<void> _resolveTicket() async {
    final l10n = AppLocalizations.of(context);
    final isp = context.isp;
    final notesCtrl = TextEditingController();

    final confirm = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.ticketResolve),
        content: TextField(
          controller: notesCtrl,
          maxLines: 3,
          decoration: InputDecoration(
            hintText: l10n.ticketResolveHint,
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(12),
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(l10n.ticketResolveConfirm),
          ),
        ],
      ),
    );

    if (confirm != true || !mounted) return;

    final svc = ref.read(ticketServiceProvider);
    final result = await svc.resolveTicket(
      ticketId: widget.id,
      completionNotes:
          notesCtrl.text.trim().isEmpty ? null : notesCtrl.text.trim(),
    );

    if (!mounted) return;

    result.fold(
      (_) {
        ref.invalidate(ticketByIdProvider(widget.id));
        ref.invalidate(ticketMessagesProvider(widget.id));
        ScaffoldMessenger.of(context)
          ..hideCurrentSnackBar()
          ..showSnackBar(
            SnackBar(
              content: Text(l10n.ticketResolved),
              behavior: SnackBarBehavior.floating,
            ),
          );
      },
      (e) {
        ScaffoldMessenger.of(context)
          ..hideCurrentSnackBar()
          ..showSnackBar(
            SnackBar(
              content: Text(e.message),
              behavior: SnackBarBehavior.floating,
            ),
          );
      },
    );
  }

  Future<void> _claimTicket(String ticketId) async {
    final l10n = AppLocalizations.of(context);
    final svc = ref.read(ticketServiceProvider);
    final result = await svc.claimTicket(ticketId);
    if (!mounted) return;
    switch (result) {
      case Success(data: _):
        ScaffoldMessenger.of(context)
          ..hideCurrentSnackBar()
          ..showSnackBar(
            SnackBar(
              content: Text(l10n.ticketClaimSuccess),
              behavior: SnackBarBehavior.floating,
            ),
          );
        // Refresh ticket data
        ref.invalidate(ticketByIdProvider(widget.id));
        ref.invalidate(ticketMessagesProvider(widget.id));
      case Failure(exception: final e):
        ScaffoldMessenger.of(context)
          ..hideCurrentSnackBar()
          ..showSnackBar(
            SnackBar(
              content: Text(e.message),
              behavior: SnackBarBehavior.floating,
            ),
          );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final isp = context.isp;
    final ticketAsync = ref.watch(ticketByIdProvider(widget.id));
    final messagesAsync = ref.watch(ticketMessagesProvider(widget.id));
    final currentUser = ref.watch(currentUserProvider);
    final dateFmt = DateFormat('d MMM yyyy HH:mm', 'id_ID');

    return Scaffold(
      appBar: AppBar(
        title: ticketAsync.when(
          loading: () => Text(l10n.myTickets),
          error: (_, __) => Text(l10n.myTickets),
          data: (t) => Text(t.subject, overflow: TextOverflow.ellipsis),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            tooltip: l10n.retry,
            onPressed: _silentRefresh,
          ),
        ],
      ),
      body: Column(
        children: [
          // Ticket status header
          ticketAsync.when(
            loading: () => const SizedBox.shrink(),
            error: (_, __) => const SizedBox.shrink(),
            data: (ticket) => Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(
                      horizontal: IspSpacing.lg, vertical: IspSpacing.sm),
                  decoration: BoxDecoration(
                    border: Border(
                        bottom: BorderSide(color: isp.borderSubtle)),
                  ),
                  child: Row(
                    children: [
                      IspStatusBadge(
                        label: l10n.ticketStatusLabel(ticket.status),
                        tone: ticket.isOpen
                            ? StatusTone.warning
                            : StatusTone.success,
                      ),
                      const SizedBox(width: IspSpacing.sm),
                      IspStatusBadge(
                        label: l10n.ticketPriorityLabel(ticket.priority),
                        tone: ticket.priority == TicketPriority.urgent
                            ? StatusTone.danger
                            : ticket.priority == TicketPriority.high
                                ? StatusTone.warning
                                : StatusTone.neutral,
                      ),
                      if (ticket.category != null &&
                          ticket.category!.isNotEmpty) ...[
                        const SizedBox(width: IspSpacing.sm),
                        IspStatusBadge(
                          label: l10n.ticketCategoryLabel(ticket.category),
                          tone: StatusTone.neutral,
                        ),
                      ],
                      const Spacer(),
                      Text(
                        dateFmt.format(ticket.createdAt),
                        style: TextStyle(
                            fontSize: 11, color: isp.textMuted),
                      ),
                    ],
                  ),
                ),
                // Subscription link
                if (ticket.subscriptionId != null)
                  Padding(
                    padding: const EdgeInsets.symmetric(
                        horizontal: IspSpacing.lg, vertical: IspSpacing.xs),
                    child: GestureDetector(
                      onTap: () => _showSubscriptionSheet(
                          context, ticket.subscriptionId!),
                      child: Row(
                        children: [
                          Icon(Icons.wifi_outlined,
                              size: 14, color: isp.accent),
                          const SizedBox(width: 6),
                          Text(
                            l10n.ticketViewSubscription,
                            style: TextStyle(
                              fontSize: 12,
                              color: isp.accent,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          const SizedBox(width: 4),
                          Icon(Icons.arrow_forward_ios,
                              size: 10, color: isp.accent),
                        ],
                      ),
                    ),
                  ),
                // Claim button (unassigned ticket, staff only)
                if (ticket.isOpen &&
                    ticket.assignedToName == null &&
                    (currentUser?.isStaff ?? false))
                  Padding(
                    padding: const EdgeInsets.symmetric(
                        horizontal: IspSpacing.lg, vertical: IspSpacing.xs),
                    child: SizedBox(
                      width: double.infinity,
                      child: FilledButton.icon(
                        onPressed: () => _claimTicket(ticket.id),
                        icon: const Icon(Icons.person_add_alt_1, size: 20),
                        label: Text(l10n.ticketClaim),
                        style: FilledButton.styleFrom(
                          backgroundColor: Colors.blue.shade600,
                          padding:
                              const EdgeInsets.symmetric(vertical: 14),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(12),
                          ),
                        ),
                      ),
                    ),
                  ),
                // Resolve button (only for open tickets)
                if (ticket.isOpen)
                  Padding(
                    padding: const EdgeInsets.symmetric(
                        horizontal: IspSpacing.lg, vertical: IspSpacing.sm),
                    child: SizedBox(
                      width: double.infinity,
                      child: FilledButton.icon(
                        onPressed: _resolveTicket,
                        icon: const Icon(Icons.check_circle_outline, size: 20),
                        label: Text(l10n.ticketResolve),
                        style: FilledButton.styleFrom(
                          backgroundColor: Colors.green.shade600,
                          padding: const EdgeInsets.symmetric(vertical: 14),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(12),
                          ),
                        ),
                      ),
                    ),
                  ),
              ],
            ),
          ),
          // Messages
          Expanded(
            child: messagesAsync.when(
              loading: () => const _MessagesSkeleton(),
              error: (e, _) => IspErrorState(
                message: e.toString(),
                onRetry: _silentRefresh,
              ),
              data: (messages) {
                if (messages.isEmpty) {
                  return IspEmptyState(
                    icon: Icons.chat_bubble_outline,
                    title: l10n.ticketNoMessages,
                    message: l10n.ticketNoMessagesHint,
                  );
                }
                return ListView.builder(
                  controller: _scrollCtrl,
                  padding: const EdgeInsets.all(IspSpacing.lg),
                  itemCount: messages.length,
                  itemBuilder: (_, i) => _MessageBubble(
                    message: messages[i],
                    dateFmt: dateFmt,
                    baseUrl: ref.watch(appConfigProvider).apiBaseUrl,
                    tokenFuture: ref.read(_authTokenProvider.future),
                    dio: ref.read(dioProvider),
                  ),
                );
              },
            ),
          ),
          // Pending attachments preview
          if (_pendingAttachments.isNotEmpty) _buildPendingAttachmentsPreview(),
          // Satisfaction survey disabled - not shown in technician app
          // ticketAsync.maybeWhen(
          //   data: (ticket) => ticket.isClosed
          //       ? TicketSatisfactionSurvey(ticketId: ticket.id)
          //       : const SizedBox.shrink(),
          //   orElse: () => const SizedBox.shrink(),
          // ),
          // Message input — hidden when closed/resolved
          ticketAsync.maybeWhen(
            data: (ticket) => (ticket.status == TicketStatus.closed ||
                    ticket.status == TicketStatus.resolved)
                ? Padding(
                    padding: const EdgeInsets.all(IspSpacing.md),
                    child: Center(
                      child: Text(
                        ticket.status == TicketStatus.closed
                            ? 'Tiket sudah ditutup'
                            : 'Tiket sudah selesai — balas untuk membuka kembali',
                        style: TextStyle(
                          color: isp.textMuted,
                          fontSize: 13,
                          fontStyle: FontStyle.italic,
                        ),
                      ),
                    ),
                  )
                : SafeArea(
                    top: false,
                    child: Container(
                      padding: const EdgeInsets.all(IspSpacing.md),
                      decoration: BoxDecoration(
                        color: isp.surface,
                        border: Border(top: BorderSide(color: isp.borderSubtle)),
                      ),
                      child: Row(
                        children: [
                          IconButton(
                            onPressed: _sending || _uploading ? null : _pickAttachment,
                            icon: const Icon(Icons.attach_file),
                            tooltip: l10n.ticketButtonAttach,
                          ),
                          const SizedBox(width: IspSpacing.xs),
                          Expanded(
                            child: TextField(
                              controller: _messageCtrl,
                              minLines: 1,
                              maxLines: 4,
                              decoration: InputDecoration(
                                hintText: l10n.ticketFieldReply,
                              ),
                              onSubmitted: (_) => _send(),
                            ),
                          ),
                          const SizedBox(width: IspSpacing.sm),
                          IconButton.filled(
                            onPressed: _sending || _uploading ? null : _send,
                            icon: (_sending || _uploading)
                                ? const SizedBox(
                                    width: 16,
                                    height: 16,
                                    child: CircularProgressIndicator(strokeWidth: 2),
                                  )
                                : const Icon(Icons.send),
                          ),
                        ],
                      ),
                    ),
                  ),
            orElse: () => const SizedBox.shrink(),
          ),
        ],
      ),
    );
  }

  Widget _buildPendingAttachmentsPreview() {
    return Container(
      constraints: const BoxConstraints(maxHeight: 120),
      padding: const EdgeInsets.symmetric(
          horizontal: IspSpacing.md, vertical: IspSpacing.sm),
      decoration: BoxDecoration(
        color: isp.surfaceTertiary,
        border: Border(top: BorderSide(color: isp.borderSubtle)),
      ),
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        itemCount: _pendingAttachments.length,
        separatorBuilder: (_, __) => const SizedBox(width: IspSpacing.sm),
        itemBuilder: (_, i) {
          final att = _pendingAttachments[i];
          return Stack(
            children: [
              // Image thumbnail or file icon
              Container(
                width: 80,
                height: 80,
                decoration: BoxDecoration(
                  color: isp.surface,
                  borderRadius: BorderRadius.circular(IspRadii.sm),
                  border: Border.all(color: isp.borderSubtle),
                ),
                clipBehavior: Clip.antiAlias,
                child: att.isImage
                    ? Image.file(
                        File(att.filePath),
                        fit: BoxFit.cover,
                        errorBuilder: (_, __, ___) => Icon(
                          Icons.image,
                          size: 32,
                          color: isp.textMuted,
                        ),
                      )
                    : Center(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(Icons.insert_drive_file_outlined,
                                size: 28, color: isp.accent),
                            const SizedBox(height: 2),
                            Text(
                              att.fileName.split('.').last.toUpperCase(),
                              style: TextStyle(
                                fontSize: 9,
                                fontWeight: FontWeight.w700,
                                color: isp.textMuted,
                              ),
                            ),
                          ],
                        ),
                      ),
              ),
              // Delete button
              Positioned(
                top: -4,
                right: -4,
                child: GestureDetector(
                  onTap: () => _removeAttachment(i),
                  child: Container(
                    width: 22,
                    height: 22,
                    decoration: BoxDecoration(
                      color: isp.danger,
                      shape: BoxShape.circle,
                    ),
                    child: const Icon(Icons.close,
                        size: 14, color: Colors.white),
                  ),
                ),
              ),
              // Filename below
              Positioned(
                bottom: 0,
                left: 0,
                right: 0,
                child: Container(
                  padding: const EdgeInsets.symmetric(
                      horizontal: 4, vertical: 2),
                  decoration: BoxDecoration(
                    color: Colors.black54,
                    borderRadius: BorderRadius.vertical(
                      bottom: Radius.circular(IspRadii.sm - 1),
                    ),
                  ),
                  child: Text(
                    att.fileName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      fontSize: 9,
                      color: Colors.white,
                    ),
                  ),
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

/// Pending file attachment before upload.
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

/// Skeleton loading state for message list.
class _MessagesSkeleton extends StatelessWidget {
  const _MessagesSkeleton();

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return ListView.builder(
      padding: const EdgeInsets.all(IspSpacing.lg),
      itemCount: 4,
      itemBuilder: (_, i) {
        final isStaff = i.isEven;
        return Align(
          alignment: isStaff ? Alignment.centerLeft : Alignment.centerRight,
          child: Container(
            margin: const EdgeInsets.symmetric(vertical: IspSpacing.xs),
            child: IspShimmer.box(
              width: isStaff ? 200 : 160,
              height: 60 + (i * 10).toDouble(),
              borderRadius: IspRadii.lg,
            ),
          ),
        );
      },
    );
  }
}

/// Chat bubble with attachments support.
class _MessageBubble extends StatelessWidget {
  const _MessageBubble({
    required this.message,
    required this.dateFmt,
    required this.baseUrl,
    required this.tokenFuture,
    required this.dio,
  });

  final TicketMessageModel message;
  final DateFormat dateFmt;
  final String baseUrl;
  final Future<String?> tokenFuture;
  final Dio dio;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final isp = context.isp;
    final isStaff = message.isFromStaff;
    return Align(
      alignment: isStaff ? Alignment.centerLeft : Alignment.centerRight,
      child: Container(
        margin: const EdgeInsets.symmetric(vertical: IspSpacing.xs),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        constraints: const BoxConstraints(maxWidth: 300),
        decoration: BoxDecoration(
          color: isStaff ? isp.surfaceTertiary : isp.accent,
          borderRadius: BorderRadius.only(
            topLeft: const Radius.circular(IspRadii.lg),
            topRight: const Radius.circular(IspRadii.lg),
            bottomLeft: Radius.circular(isStaff ? 2 : IspRadii.lg),
            bottomRight: Radius.circular(isStaff ? IspRadii.lg : 2),
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            // Author label
            if (isStaff)
              Padding(
                padding: const EdgeInsets.only(bottom: IspSpacing.xs),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.shield,
                        size: 12, color: isp.accent),
                    const SizedBox(width: IspSpacing.xs),
                    Text(
                      l10n.ticketAuthorLabel(
                        message.authorName,
                        isCurrentUser: false,
                      ),
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        color: isp.accent,
                      ),
                    ),
                  ],
                ),
              ),
            // Message body
            if (message.body.isNotEmpty)
              Text(
                message.body,
                style: TextStyle(
                    color: isStaff ? isp.textPrimary : Colors.white),
              ),
            // Attachments
            if (message.attachments.isNotEmpty) ...[
              const SizedBox(height: IspSpacing.sm),
              ...message.attachments.map(
                (att) => _AttachmentWidget(
                  attachment: att,
                  baseUrl: baseUrl,
                  tokenFuture: tokenFuture,
                  isStaff: isStaff,
                  dio: dio,
                ),
              ),
            ],
            const SizedBox(height: IspSpacing.xs),
            Text(
              dateFmt.format(message.createdAt),
              style: TextStyle(
                fontSize: 10,
                color: isStaff ? isp.textMuted : Colors.white70,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Displays a single attachment — image inline, other files as download link.
class _AttachmentWidget extends StatefulWidget {
  const _AttachmentWidget({
    required this.attachment,
    required this.baseUrl,
    required this.tokenFuture,
    required this.isStaff,
    required this.dio,
  });

  final TicketAttachmentModel attachment;
  final String baseUrl;
  final Future<String?> tokenFuture;
  final bool isStaff;
  final Dio dio;

  @override
  State<_AttachmentWidget> createState() => _AttachmentWidgetState();
}

class _AttachmentWidgetState extends State<_AttachmentWidget> {
  bool _downloadingVideo = false;

  TicketAttachmentModel get attachment => widget.attachment;
  String get baseUrl => widget.baseUrl;
  Future<String?> get tokenFuture => widget.tokenFuture;
  bool get isStaff => widget.isStaff;
  Dio get dio => widget.dio;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final isp = context.isp;
    final fileUrl =
        '$baseUrl/api/storage/files/${attachment.id}/ticket-content';

    if (attachment.isImage) {
      return FutureBuilder<String?>(
        future: tokenFuture,
        builder: (context, snap) {
          if (snap.connectionState == ConnectionState.waiting) {
            return _buildImageLoading(context);
          }
          final token = snap.data;
          if (token == null || token.isEmpty) {
            return _buildImageError(context, l10n.ticketErrorSessionExpired);
          }
          // Use CachedNetworkImage for disk + memory caching.
          // Custom cacheKey excludes the token so cache survives token rotation.
          return CachedNetworkImage(
            imageUrl: fileUrl,
            httpHeaders: {'Authorization': 'Bearer $token'},
            cacheKey: 'ticket-attachment-${attachment.id}',
            width: 220,
            fit: BoxFit.cover,
            placeholder: (_, __) => _buildImageLoading(context),
            errorWidget: (_, __, ___) =>
                _buildImageError(context, l10n.ticketErrorLoadFailed),
            imageBuilder: (context, provider) => GestureDetector(
              onTap: () => _openFullImageUrl(context, fileUrl, token),
              child: ClipRRect(
                borderRadius: BorderRadius.circular(IspRadii.sm),
                child: Image(
                  image: provider,
                  width: 220,
                  fit: BoxFit.cover,
                ),
              ),
            ),
          );
        },
      );
    }

    // Video: inline tile that downloads to gallery on tap (no browser).
    if (attachment.isVideo) {
      return FutureBuilder<String?>(
        future: tokenFuture,
        builder: (context, snap) {
          final token = snap.data ?? '';
          // Token embedded in URL (same pattern as non-image handler below).
          final downloadUrl = token.isEmpty
              ? fileUrl
              : '$fileUrl?token=$token';
          return _buildVideoTile(
            context: context,
            attachment: attachment,
            videoUrl: downloadUrl,
            token: token,
          );
        },
      );
    }

    // Non-image / non-video: download to temp then open with native app.
    return FutureBuilder<String?>(
      future: tokenFuture,
      builder: (_, snap) {
        final token = snap.data ?? '';
        final downloadUrl =
            '$baseUrl/api/storage/files/${attachment.id}/ticket-content';
        return Padding(
          padding: const EdgeInsets.only(bottom: IspSpacing.xs),
          child: InkWell(
            onTap: _downloadingVideo
                ? null
                : () => _openAttachmentFile(
                      context: context,
                      fileUrl: downloadUrl,
                      token: token,
                      attachment: attachment,
                    ),
            borderRadius: BorderRadius.circular(IspRadii.sm),
            child: Container(
              padding: const EdgeInsets.all(IspSpacing.sm),
              decoration: BoxDecoration(
                color: (isStaff ? Colors.black : Colors.white).withOpacity(0.1),
                borderRadius: BorderRadius.circular(IspRadii.sm),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    _fileIcon(attachment.contentType),
                    size: 20,
                    color: isStaff ? isp.textPrimary : Colors.white,
                  ),
                  const SizedBox(width: IspSpacing.sm),
                  Flexible(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          attachment.originalName,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            fontSize: 12,
                            color:
                                isStaff ? isp.textPrimary : Colors.white,
                          ),
                        ),
                        Text(
                          attachment.humanSize,
                          style: TextStyle(
                            fontSize: 10,
                            color: isStaff
                                ? isp.textMuted
                                : Colors.white60,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: IspSpacing.xs),
                  Icon(
                    _downloadingVideo
                        ? Icons.hourglass_empty
                        : Icons.open_in_new,
                    size: 16,
                    color: isStaff ? isp.textMuted : Colors.white60,
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  IconData _fileIcon(String contentType) {
    if (contentType.startsWith('image/')) return Icons.image;
    if (contentType.startsWith('video/')) return Icons.videocam;
    if (contentType.startsWith('audio/')) return Icons.audiotrack;
    if (contentType.contains('pdf')) return Icons.picture_as_pdf;
    if (contentType.contains('zip') || contentType.contains('rar'))
      return Icons.archive;
    return Icons.attach_file;
  }

  Widget _buildImageLoading(BuildContext context) {
    final isp = context.isp;
    return Container(
      width: 220,
      height: 120,
      alignment: Alignment.center,
      child: CircularProgressIndicator(
        strokeWidth: 2,
        color: isStaff ? isp.accent : Colors.white70,
      ),
    );
  }

  /// Error state with retry button so user isn't stuck on broken image.
  Widget _buildImageError(BuildContext context, String message) {
    final isp = context.isp;
    return Container(
      width: 220,
      height: 80,
      alignment: Alignment.center,
      padding: const EdgeInsets.symmetric(horizontal: IspSpacing.sm),
      decoration: BoxDecoration(
        color: (isStaff ? Colors.black : Colors.white).withOpacity(0.05),
        borderRadius: BorderRadius.circular(IspRadii.sm),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.broken_image,
            color: isStaff ? isp.textMuted : Colors.white54,
          ),
          const SizedBox(height: 4),
          Text(
            message,
            textAlign: TextAlign.center,
            style: TextStyle(
              fontSize: 10,
              color: isStaff ? isp.textMuted : Colors.white54,
            ),
          ),
          const SizedBox(height: 4),
          TextButton.icon(
            onPressed: () {
              // Clear cache for this key, force CachedNetworkImage to retry
              DefaultCacheManager()
                  .removeFile('ticket-attachment-${attachment.id}');
              (context as Element).markNeedsBuild();
            },
            icon: const Icon(Icons.refresh, size: 12),
            label: const Text('Coba lagi', style: TextStyle(fontSize: 10)),
            style: TextButton.styleFrom(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 0),
              minimumSize: Size.zero,
              tapTargetSize: MaterialTapTargetSize.shrinkWrap,
            ),
          ),
        ],
      ),
    );
  }

  /// Open full-size image using the same cached image provider.
  void _openFullImageUrl(BuildContext context, String url, String token) {
    showDialog(
      context: context,
      builder: (_) => Dialog(
        insetPadding: const EdgeInsets.all(IspSpacing.lg),
        child: GestureDetector(
          onTap: () => Navigator.of(context).pop(),
          child: InteractiveViewer(
            child: CachedNetworkImage(
              imageUrl: url,
              httpHeaders: {'Authorization': 'Bearer $token'},
              cacheKey: 'ticket-attachment-${attachment.id}-full',
              fit: BoxFit.contain,
              placeholder: (_, __) => const Padding(
                padding: EdgeInsets.all(32),
                child: CircularProgressIndicator(),
              ),
              errorWidget: (_, __, ___) => const Padding(
                padding: EdgeInsets.all(32),
                child: Icon(Icons.broken_image, size: 48),
              ),
            ),
          ),
        ),
      ),
    );
  }

  /// Inline video tile — tap downloads to temp dir then opens with the
  /// native app on the device (video player, PDF viewer, etc.) via url_launcher.
  /// No browser, no gallery — file stays in temp and is opened directly.
  Widget _buildVideoTile({
    required BuildContext context,
    required TicketAttachmentModel attachment,
    required String videoUrl,
    required String token,
  }) {
    final isp = context.isp;
    return Padding(
      padding: const EdgeInsets.only(bottom: IspSpacing.xs),
      child: InkWell(
        onTap: _downloadingVideo
            ? null
            : () => _openAttachmentFile(
                  context: context,
                  fileUrl: videoUrl,
                  token: token,
                  attachment: attachment,
                ),
        borderRadius: BorderRadius.circular(IspRadii.sm),
        child: Container(
          width: 220,
          padding: const EdgeInsets.all(IspSpacing.md),
          decoration: BoxDecoration(
            color: (isStaff ? Colors.black : Colors.white).withOpacity(0.08),
            borderRadius: BorderRadius.circular(IspRadii.sm),
            border: Border.all(
              color: isp.borderSubtle.withOpacity(0.3),
            ),
          ),
          child: Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: isp.accent.withOpacity(0.15),
                  shape: BoxShape.circle,
                ),
                child: _downloadingVideo
                    ? Padding(
                        padding: const EdgeInsets.all(10),
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: isp.accent,
                        ),
                      )
                    : Icon(
                        Icons.play_circle_outline,
                        color: isp.accent,
                      ),
              ),
              const SizedBox(width: IspSpacing.sm),
              Flexible(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      attachment.originalName,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12,
                        color: isStaff ? isp.textPrimary : Colors.white,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      _downloadingVideo
                          ? 'Mengunduh…'
                          : 'Tap untuk buka',
                      style: TextStyle(
                        fontSize: 10,
                        color: isp.textMuted,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      attachment.humanSize,
                      style: TextStyle(
                        fontSize: 10,
                        color: isp.textMuted,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// Download an attachment to a persistent app-private cache directory
  /// (NOT the OS temp dir — that can be wiped at any time) and open it
  /// with the native app on the device via share_plus.
  ///
  /// First download: full HTTP fetch → saved to cache → opened.
  /// Subsequent taps: file already exists in cache → opened directly,
  /// no re-download. Cache survives app restarts; only cleared by user
  /// clearing app data or `path_provider`'s cache eviction (very rare).
  Future<void> _openAttachmentFile({
    required BuildContext context,
    required String fileUrl,
    required String token,
    required TicketAttachmentModel attachment,
  }) async {
    if (_downloadingVideo) return;
    setState(() => _downloadingVideo = true);
    final messenger = ScaffoldMessenger.of(context);
    try {
      // Use application support directory (persistent, app-private).
      // Sub-folder per attachment id so files with same originalName
      // don't collide across different tickets.
      final baseDir = await getApplicationSupportDirectory();
      final cacheDir = Directory('${baseDir.path}/ticket_attachments');
      if (!await cacheDir.exists()) {
        await cacheDir.create(recursive: true);
      }
      final safeName = attachment.originalName.isNotEmpty
          ? attachment.originalName
          : 'file_${attachment.id}';
      final cachedPath = '${cacheDir.path}/${attachment.id}_$safeName';

      // ── Cache hit: skip download ──
      final cached = File(cachedPath);
      if (await cached.exists()) {
        debugPrint('[ticket-attachment] cache hit: $cachedPath');
        await Share.shareXFiles(
          [XFile(cachedPath, mimeType: attachment.resolvedContentType)],
          text: attachment.originalName,
        );
        return;
      }

      // ── Cache miss: download once ──
      debugPrint('[ticket-attachment] downloading: $fileUrl → $cachedPath');
      await dio.download(
        fileUrl,
        cachedPath,
        options: token.isEmpty
            ? null
            : Options(headers: {'Authorization': 'Bearer $token'}),
      );

      final file = File(cachedPath);
      if (!await file.exists()) {
        throw Exception('File tidak ditemukan setelah unduh');
      }

      // Open with native app via share_plus — it handles FileProvider/content://
      // so the file is accessible to other apps. Shows app chooser (open with...).
      await Share.shareXFiles(
        [XFile(cachedPath, mimeType: attachment.resolvedContentType)],
        text: attachment.originalName,
      );
    } on DioException catch (e) {
      debugPrint('[ticket-attachment] download failed: ${e.message}');
      if (!mounted) return;
      messenger.showSnackBar(
        SnackBar(content: Text('Gagal unduh: ${e.message ?? 'network error'}')),
      );
    } catch (e) {
      debugPrint('[ticket-attachment] open failed: $e');
      if (!mounted) return;
      messenger.showSnackBar(
        SnackBar(content: Text('Gagal buka file: $e')),
      );
    } finally {
      if (mounted) setState(() => _downloadingVideo = false);
    }
  }
}

// ─── Subscription Info Bottom Sheet (technician view) ──────────

class _SubscriptionInfoSheet extends ConsumerWidget {
  const _SubscriptionInfoSheet({required this.subscriptionId});
  final String subscriptionId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final subAsync = ref.watch(_subscriptionByIdProvider(subscriptionId));

    return DraggableScrollableSheet(
      initialChildSize: 0.45,
      minChildSize: 0.3,
      maxChildSize: 0.7,
      expand: false,
      builder: (ctx, scrollCtrl) => SingleChildScrollView(
        controller: scrollCtrl,
        padding: const EdgeInsets.all(20),
        child: subAsync.when(
          loading: () => const Center(
            child: Padding(
              padding: EdgeInsets.all(32),
              child: CircularProgressIndicator(),
            ),
          ),
          error: (e, _) => Center(
            child: Padding(
              padding: const EdgeInsets.all(32),
              child: Column(
                children: [
                  Icon(Icons.error_outline,
                      size: 36, color: isp.textMuted),
                  const SizedBox(height: 8),
                  Text('Gagal memuat data',
                      style: TextStyle(color: isp.textMuted)),
                ],
              ),
            ),
          ),
          data: (sub) => Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Center(
                child: Container(
                  width: 40,
                  height: 4,
                  decoration: BoxDecoration(
                    color: isp.border,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
              const SizedBox(height: 16),
              Text(
                l10n.subscriptionDetail,
                style: TextStyle(
                  fontSize: 18,
                  fontWeight: FontWeight.w700,
                  color: isp.textPrimary,
                ),
              ),
              const SizedBox(height: 16),
              _SheetInfoRow(isp, l10n.internetPackage, sub.packageName),
              _SheetInfoRow(isp, l10n.location, sub.locationLabel),
              _SheetInfoRow(isp, l10n.router, sub.routerName),
              _SheetInfoRow(isp, l10n.price,
                  'Rp ${NumberFormat('#,###').format(sub.price)}'),
              _SheetInfoRow(isp, l10n.cycle, sub.billingCycle),
              _SheetInfoRow(isp, 'Status', sub.status.name),
              if (sub.startsAt != null)
                _SheetInfoRow(
                    isp, l10n.startsAt, _fmtDate(sub.startsAt!)),
              if (sub.endsAt != null)
                _SheetInfoRow(isp, l10n.endsAt, _fmtDate(sub.endsAt!)),
              if (sub.notes != null && sub.notes!.isNotEmpty) ...[
                const SizedBox(height: 8),
                Text(
                  l10n.notes,
                  style: TextStyle(
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                      color: isp.textMuted),
                ),
                const SizedBox(height: 4),
                Text(
                  sub.notes!,
                  style:
                      TextStyle(fontSize: 13, color: isp.textSecondary),
                ),
              ],
              const SizedBox(height: 24),
            ],
          ),
        ),
      ),
    );
  }

  String _fmtDate(DateTime dt) =>
      DateFormat('dd/MM/yyyy').format(dt);
}

final _subscriptionByIdProvider =
    FutureProvider.family<SubscriptionModel, String>((ref, id) async {
  final svc = ref.watch(subscriptionServiceProvider);
  final user = ref.watch(currentUserProvider);
  // Staff/technician: use admin endpoint (requires billing:read)
  // Customer: use portal endpoint
  if (user != null && user.isStaff) {
    final res = await svc.getByIdAdmin(id);
    return res.getOrThrow();
  }
  final res = await svc.getById(id);
  return res.getOrThrow();
});

class _SheetInfoRow extends StatelessWidget {
  const _SheetInfoRow(this.isp, this.label, this.value);
  final IspThemeColors isp;
  final String label;
  final String? value;

  @override
  Widget build(BuildContext context) {
    if (value == null || value!.isEmpty) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 80,
            child: Text(
              label,
              style: TextStyle(fontSize: 13, color: isp.textMuted),
            ),
          ),
          Expanded(
            child: Text(
              value!,
              style: TextStyle(fontSize: 13, color: isp.textPrimary),
            ),
          ),
        ],
      ),
    );
  }
}
