// Sprint 3: ticket action providers — start / upload photo / resolve.
//
// These power the technician's workflow:
//   1. Tech opens ticket detail
//   2. Taps "Mulai" → ticketActionController.startTicket(id)
//   3. Goes to site, takes photos → addPhoto + uploadPhoto (multipart)
//   4. Taps "Selesaikan" → resolve dialog (notes + photos + signature)
//   5. resolveTicket submits completion proof to backend

import 'dart:io';
import 'dart:typed_data';

import 'package:api_client/api_client.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'service_providers.dart';
import 'ticket_providers.dart';

/// Holds a single in-progress photo capture before it's uploaded to the server.
/// [localPath] is the path on the device; [remoteId] is set after upload.
@immutable
class PendingPhoto {
  const PendingPhoto({
    required this.localPath,
    required this.capturedAt,
    this.remoteId,
    this.uploading = false,
    this.error,
  });

  final String localPath;
  final DateTime capturedAt;
  final String? remoteId;
  final bool uploading;
  final String? error;

  bool get isUploaded => remoteId != null;

  PendingPhoto copyWith({
    String? remoteId,
    bool? uploading,
    String? error,
    bool clearError = false,
  }) {
    return PendingPhoto(
      localPath: localPath,
      capturedAt: capturedAt,
      remoteId: remoteId ?? this.remoteId,
      uploading: uploading ?? this.uploading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// State for a ticket resolve session: notes + photo list + optional signature.
@immutable
class ResolveDraft {
  const ResolveDraft({
    this.notes = '',
    this.photos = const [],
    this.signatureBytes,
  });

  final String notes;
  final List<PendingPhoto> photos;
  final Uint8List? signatureBytes;

  bool get hasSignature => signatureBytes != null;
  bool get hasPhotos => photos.any((p) => p.isUploaded);
  bool get isReady => notes.trim().isNotEmpty || hasPhotos || hasSignature;

  ResolveDraft copyWith({
    String? notes,
    List<PendingPhoto>? photos,
    Uint8List? signatureBytes,
    bool clearSignature = false,
  }) {
    return ResolveDraft(
      notes: notes ?? this.notes,
      photos: photos ?? this.photos,
      signatureBytes: clearSignature ? null : (signatureBytes ?? this.signatureBytes),
    );
  }
}

class ResolveDraftController extends StateNotifier<ResolveDraft> {
  ResolveDraftController() : super(const ResolveDraft());

  void setNotes(String value) {
    state = state.copyWith(notes: value);
  }

  void addPhoto(String localPath) {
    state = state.copyWith(
      photos: [
        ...state.photos,
        PendingPhoto(localPath: localPath, capturedAt: DateTime.now()),
      ],
    );
  }

  void removePhoto(int index) {
    if (index < 0 || index >= state.photos.length) return;
    final updated = [...state.photos]..removeAt(index);
    state = state.copyWith(photos: updated);
  }

  Future<void> uploadAllPhotos(WidgetRef ref, String ticketId) async {
    final svc = ref.read(ticketServiceProvider);
    final pending = state.photos.where((p) => !p.isUploaded && !p.uploading).toList();
    for (final photo in pending) {
      final idx = state.photos.indexWhere((p) => p.localPath == photo.localPath);
      if (idx < 0) continue;
      state = state.copyWith(
        photos: [
          ...state.photos.sublist(0, idx),
          state.photos[idx].copyWith(uploading: true, clearError: true),
          ...state.photos.sublist(idx + 1),
        ],
      );
      try {
        final res = await svc.uploadPhoto(
          ticketId: ticketId,
          filePath: photo.localPath,
        );
        final uploaded = res.fold(
          (r) => r,
          (_) => null,
        );
        if (uploaded == null) {
          state = state.copyWith(
            photos: [
              ...state.photos.sublist(0, idx),
              state.photos[idx].copyWith(uploading: false, error: 'Upload gagal'),
              ...state.photos.sublist(idx + 1),
            ],
          );
          continue;
        }
        state = state.copyWith(
          photos: [
            ...state.photos.sublist(0, idx),
            state.photos[idx].copyWith(uploading: false, remoteId: uploaded.id),
            ...state.photos.sublist(idx + 1),
          ],
        );
      } catch (e) {
        state = state.copyWith(
          photos: [
            ...state.photos.sublist(0, idx),
            state.photos[idx].copyWith(uploading: false, error: e.toString()),
            ...state.photos.sublist(idx + 1),
          ],
        );
      }
    }
  }

  void setSignature(Uint8List bytes) {
    state = state.copyWith(signatureBytes: bytes);
  }

  void clearSignature() {
    state = state.copyWith(clearSignature: true);
  }

  void reset() {
    state = const ResolveDraft();
  }
}

final resolveDraftProvider =
    StateNotifierProvider.autoDispose<ResolveDraftController, ResolveDraft>(
  (ref) => ResolveDraftController(),
);

/// Single-action ticket controller — used for start/upload/resolve.
/// Returns Result<TicketModel> on success, or null on failure (with snackbar
/// shown by the caller).
class TicketActionController {
  TicketActionController(this.ref);
  final Ref ref;

  Future<TicketModel?> start(String ticketId) async {
    final res = await ref.read(ticketServiceProvider).startTicket(ticketId);
    return res.fold(
      (t) {
        // Refresh ticket cache so detail screen reflects new status.
        ref.invalidate(ticketByIdProvider(ticketId));
        ref.invalidate(myTicketsProvider);
        ref.invalidate(ticketStatsProvider);
        return t;
      },
      (e) {
        debugPrint('[TicketAction.start] failed: ${e.message}');
        return null;
      },
    );
  }

  Future<TicketModel?> resolve({
    required String ticketId,
    required ResolveDraft draft,
  }) async {
    final svc = ref.read(ticketServiceProvider);
    final uploadedIds = draft.photos
        .where((p) => p.isUploaded)
        .map((p) => p.remoteId!)
        .toList();

    // If signature is attached, upload it as a "photo" too — backend
    // treats any file_record as a generic attachment; signatureFileId
    // is just metadata on the ticket row.
    String? signatureFileId;
    if (draft.signatureBytes != null) {
      // Write bytes to a temp file so MultipartFile.fromFile can read it.
      final tmp = await _writeTempBytes(draft.signatureBytes!, 'signature.png');
      final res = await svc.uploadPhoto(
        ticketId: ticketId,
        filePath: tmp.path,
        filename: 'signature.png',
      );
      final r = res.fold((v) => v, (_) => null);
      if (r == null) {
        try {
          await tmp.delete();
        } catch (_) {}
        return null;
      }
      signatureFileId = r.id;
      try {
        await tmp.delete();
      } catch (_) {}
    }

    final res = await svc.resolveTicket(
      ticketId: ticketId,
      completionNotes: draft.notes.trim().isEmpty ? null : draft.notes.trim(),
      photoFileIds: uploadedIds.isEmpty ? null : uploadedIds,
      signatureFileId: signatureFileId,
    );

    return res.fold(
      (t) {
        ref.invalidate(ticketByIdProvider(ticketId));
        ref.invalidate(myTicketsProvider);
        ref.invalidate(ticketStatsProvider);
        return t;
      },
      (e) {
        debugPrint('[TicketAction.resolve] failed: ${e.message}');
        return null;
      },
    );
  }

  Future<File> _writeTempBytes(Uint8List bytes, String filename) async {
    final dir = Directory.systemTemp;
    final f = File('${dir.path}/$filename');
    await f.writeAsBytes(bytes, flush: true);
    return f;
  }
}

final ticketActionControllerProvider = Provider<TicketActionController>((ref) {
  return TicketActionController(ref);
});