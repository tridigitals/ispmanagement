import 'dart:convert';
import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:image_picker/image_picker.dart';
import 'package:permission_handler/permission_handler.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/app_config.dart';
import '../../services/auth_providers.dart';
import '../../utils/form_validators.dart';

class EditProfileScreen extends ConsumerStatefulWidget {
  const EditProfileScreen({super.key});
  @override
  ConsumerState<EditProfileScreen> createState() => _State();
}

class _State extends ConsumerState<EditProfileScreen> {
  late final IspThemeColors isp;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }

  final _form = GlobalKey<FormState>();
  late final TextEditingController _name;
  late final TextEditingController _email;
  late final TextEditingController _phone;
  bool _saving = false;
  bool _uploadingAvatar = false;
  String? _avatarUrl;
  final _imagePicker = ImagePicker();

  @override
  void initState() {
    super.initState();
    final user = ref.read(currentUserProvider);
    _name = TextEditingController(text: user?.name ?? '');
    _email = TextEditingController(text: user?.email ?? '');
    _phone = TextEditingController(text: user?.phone ?? '');
    _avatarUrl = user?.avatarUrl;
  }

  @override
  void dispose() {
    _name.dispose();
    _email.dispose();
    _phone.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _saving = true);
    final res = await ref.read(authControllerProvider.notifier).updateProfile(
          name: _name.text.trim(),
          email: _email.text.trim(),
          phone: _phone.text.trim(),
        );
    if (!mounted) return;
    setState(() => _saving = false);
    res.fold(
      (_) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(AppLocalizations.of(context).profileUpdated),
          ),
        );
        context.pop();
      },
      (err) => ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(err.message),
          backgroundColor: isp.danger,
        ),
      ),
    );
  }

  /// Show bottom sheet to pick source: camera or gallery
  Future<void> _pickAvatar() async {
    final source = await showModalBottomSheet<_AvatarSource>(
      context: context,
      backgroundColor: isp.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(IspRadii.lg)),
      ),
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              leading: Icon(Icons.photo_camera_outlined, color: isp.accent),
              title: const Text('Ambil Foto'),
              onTap: () => Navigator.pop(ctx, _AvatarSource.camera),
            ),
            ListTile(
              leading: Icon(Icons.photo_library_outlined, color: isp.accent),
              title: const Text('Pilih dari Galeri'),
              onTap: () => Navigator.pop(ctx, _AvatarSource.gallery),
            ),
          ],
        ),
      ),
    );
    if (source == null || !mounted) return;

    File? file;
    if (source == _AvatarSource.camera) {
      file = await _captureFromCamera();
    } else {
      file = await _pickFromGallery();
    }
    if (file == null || !mounted) return;
    await _uploadAvatar(file);
  }

  Future<File?> _captureFromCamera() async {
    // Explicit permission request before invoking camera.
    final status = await Permission.camera.request();
    if (status.isPermanentlyDenied) {
      if (!mounted) return null;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: const Text(
            'Izin kamera ditolak permanen. Buka Settings untuk mengaktifkan.',
          ),
          backgroundColor: isp.danger,
          action: SnackBarAction(
          label: 'Settings',
          textColor: Colors.white,
          onPressed: () => openAppSettings(),
          ),
        ),
      );
      return null;
    }
    if (!status.isGranted) return null;

    try {
      final picked = await _imagePicker.pickImage(
        source: ImageSource.camera,
        maxWidth: 1024,
        maxHeight: 1024,
        imageQuality: 85,
      );
      if (picked == null) return null;
      return File(picked.path);
    } catch (e) {
      if (!mounted) return null;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Gagal membuka kamera: $e'),
          backgroundColor: isp.danger,
        ),
      );
      return null;
    }
  }

  Future<File?> _pickFromGallery() async {
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.image,
        allowMultiple: false,
        withData: false,
      );
      if (result == null || result.files.isEmpty) return null;
      final path = result.files.single.path;
      if (path == null) return null;
      return File(path);
    } catch (e) {
      if (!mounted) return null;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Gagal membuka galeri: $e'),
          backgroundColor: isp.danger,
        ),
      );
      return null;
    }
  }

  Future<void> _uploadAvatar(File file) async {
    setState(() => _uploadingAvatar = true);
    try {
      final bytes = await file.readAsBytes();
      final base64 = base64Encode(bytes);
      final res = await ref
          .read(authControllerProvider.notifier)
          .uploadAvatar(base64);
      if (!mounted) return;
      res.fold(
        (url) {
          setState(() => _avatarUrl = url);
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text('Foto profil berhasil diperbarui')),
          );
        },
        (err) => ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Gagal upload foto: ${err.message}'),
            backgroundColor: isp.danger,
          ),
        ),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Gagal membaca file: $e'),
          backgroundColor: isp.danger,
        ),
      );
    } finally {
      if (mounted) setState(() => _uploadingAvatar = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.editProfile)),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(IspSpacing.lg),
          child: Form(
            key: _form,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const SizedBox(height: IspSpacing.lg),
                Center(
                  child: GestureDetector(
                    onTap: _uploadingAvatar ? null : _pickAvatar,
                    child: Stack(
                      children: [
                        CircleAvatar(
                          radius: 48,
                          backgroundColor: isp.accentSurface,
                          backgroundImage:
                              _avatarUrl != null && _avatarUrl!.isNotEmpty
                                  ? NetworkImage(
                                      _buildAbsoluteUrl(_avatarUrl!),
                                    )
                                  : null,
                          child: _avatarUrl == null || _avatarUrl!.isEmpty
                              ? Icon(
                                  Icons.person,
                                  size: 48,
                                  color: isp.accent,
                                )
                              : null,
                        ),
                        if (_uploadingAvatar)
                          Positioned.fill(
                            child: Container(
                              decoration: BoxDecoration(
                                color: Colors.black54,
                                shape: BoxShape.circle,
                              ),
                              child: const Center(
                                child: SizedBox(
                                  width: 24,
                                  height: 24,
                                  child: CircularProgressIndicator(
                                    strokeWidth: 2,
                                    color: Colors.white,
                                  ),
                                ),
                              ),
                            ),
                          ),
                        Positioned(
                          bottom: 0,
                          right: 0,
                          child: Container(
                            decoration: BoxDecoration(
                              color: isp.accent,
                              shape: BoxShape.circle,
                            ),
                            padding: const EdgeInsets.all(6),
                            child: const Icon(
                              Icons.camera_alt_outlined,
                              color: Colors.white,
                              size: 16,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: IspSpacing.xxl),
                TextFormField(
                  controller: _name,
                  enabled: false,
                  textInputAction: TextInputAction.next,
                  decoration: InputDecoration(
                    labelText: l10n.fullName,
                    prefixIcon: const Icon(Icons.person_outline),
                    helperText: 'Nama hanya dapat diubah oleh admin',
                  ),
                  validator: (v) =>
                      Validators.required(v, label: l10n.fullName),
                ),
                const SizedBox(height: IspSpacing.md),
                TextFormField(
                  controller: _email,
                  enabled: false,
                  keyboardType: TextInputType.emailAddress,
                  textInputAction: TextInputAction.next,
                  decoration: InputDecoration(
                    labelText: l10n.email,
                    prefixIcon: const Icon(Icons.email_outlined),
                    helperText: 'Email hanya dapat diubah oleh admin',
                  ),
                  validator: Validators.email,
                ),
                const SizedBox(height: IspSpacing.md),
                TextFormField(
                  controller: _phone,
                  keyboardType: TextInputType.phone,
                  decoration: InputDecoration(
                    labelText: l10n.phone,
                    prefixIcon: const Icon(Icons.phone_outlined),
                  ),
                  validator: Validators.phone,
                ),
                const SizedBox(height: IspSpacing.xxl),
                IspPrimaryButton(
                  label: l10n.save,
                  loading: _saving,
                  onPressed: _save,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  /// Convert relative URL like `/api/auth/avatar/...` to absolute URL
  /// using the app's API base from config.
  String _buildAbsoluteUrl(String url) {
    if (url.startsWith('http://') || url.startsWith('https://')) return url;
    final apiBase = ref.read(appConfigProvider).apiBaseUrl;
    return '$apiBase$url';
  }
}

enum _AvatarSource { camera, gallery }
