import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:dio/dio.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_cache_manager/flutter_cache_manager.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:intl/intl.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../services/app_config.dart';
import '../../services/auth_providers.dart';
import '../../services/service_providers.dart';
import 'ticket_satisfaction_survey.dart';

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

  /// Pending attachments: list of (filePath, fileName, contentType).
  final List<_PendingAttachment> _pendingAttachments = [];

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _autoRefreshTimer = Timer.periodic(
      const Duration(seconds: 30),
      (_) => _silentRefresh(),
    );
  }

  @override
  void dispose() {
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

  Future<void> _pickFile() async {
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
        SnackBar(content: Text('Gagal memilih file: $e')),
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
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Gagal mengirim: $e')),
      );
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    final ticketAsync = ref.watch(ticketByIdProvider(widget.id));
    final messagesAsync = ref.watch(ticketMessagesProvider(widget.id));
    final dateFmt = DateFormat('d MMM yyyy HH:mm', 'id_ID');

    return Scaffold(
      appBar: AppBar(
        title: ticketAsync.maybeWhen(
          data: (t) => Text(t.subject, overflow: TextOverflow.ellipsis),
          orElse: () => const Text('Tiket'),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
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
                        label: ticket.statusLabel(),
                        tone: ticket.isOpen
                            ? StatusTone.warning
                            : StatusTone.success,
                      ),
                      const SizedBox(width: IspSpacing.sm),
                      IspStatusBadge(
                        label: ticket.priorityLabel(),
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
                          label: ticket.categoryLabel(),
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
                      onTap: () => GoRouter.of(context)
                          .push('/subscriptions/${ticket.subscriptionId}'),
                      child: Row(
                        children: [
                          Icon(Icons.wifi_outlined,
                              size: 14, color: isp.accent),
                          const SizedBox(width: 6),
                          Text(
                            'Lihat langganan terkait',
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
                  return const IspEmptyState(
                    icon: Icons.chat_bubble_outline,
                    title: 'Belum ada pesan',
                    message:
                        'Kirim pesan pertama Anda untuk memulai percakapan',
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
          // Satisfaction survey (shown when ticket is closed/resolved)
          ticketAsync.maybeWhen(
            data: (ticket) => ticket.isClosed
                ? TicketSatisfactionSurvey(ticketId: ticket.id)
                : const SizedBox.shrink(),
            orElse: () => const SizedBox.shrink(),
          ),
          // Message input
          SafeArea(
            top: false,
            child: Container(
              padding: const EdgeInsets.all(IspSpacing.md),
              decoration: BoxDecoration(
                color: IspColors.bgSecondary,
                border: Border(top: BorderSide(color: isp.borderSubtle)),
              ),
              child: Row(
                children: [
                  // Attachment button
                  IconButton(
                    onPressed: _sending || _uploading ? null : _pickFile,
                    icon: const Icon(Icons.attach_file),
                    tooltip: 'Lampirkan file',
                  ),
                  const SizedBox(width: IspSpacing.xs),
                  Expanded(
                    child: TextField(
                      controller: _messageCtrl,
                      minLines: 1,
                      maxLines: 4,
                      decoration:
                          const InputDecoration(hintText: 'Tulis pesan...'),
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
        ],
      ),
    );
  }

  Widget _buildPendingAttachmentsPreview() {
    return Container(
      constraints: const BoxConstraints(maxHeight: 100),
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


    final isp = context.isp;    final isStaff = message.isFromStaff;
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
                      message.authorName,
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
class _AttachmentWidget extends StatelessWidget {
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
  Widget build(BuildContext context) {


    final isp = context.isp;    final fileUrl =
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
            return _buildImageError(context, 'Sesi berakhir, login ulang');
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
                _buildImageError(context, 'Gagal memuat'),
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

    // Non-image: file download link (URL with token query)
    return FutureBuilder<String?>(
      future: tokenFuture,
      builder: (_, snap) {
        final token = snap.data ?? '';
        final downloadUrl =
            '$baseUrl/api/storage/files/${attachment.id}/ticket-content?token=$token';
        return Padding(
          padding: const EdgeInsets.only(bottom: IspSpacing.xs),
          child: InkWell(
            onTap: () => launchUrl(Uri.parse(downloadUrl),
                mode: LaunchMode.externalApplication),
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
                    Icons.download,
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
}
