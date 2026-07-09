import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../utils/form_validators.dart';

class ForgotPasswordScreen extends ConsumerStatefulWidget {
  const ForgotPasswordScreen({super.key});
  @override
  ConsumerState<ForgotPasswordScreen> createState() => _State();
}

class _State extends ConsumerState<ForgotPasswordScreen> {
  final _form = GlobalKey<FormState>();
  final _email = TextEditingController();
  final _reason = TextEditingController();
  bool _sending = false;
  String? _done;
  bool _isError = false;

  @override
  void dispose() {
    _email.dispose();
    _reason.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (!_form.currentState!.validate()) return;
    setState(() {
      _sending = true;
      _done = null;
    });
    final res = await ref.read(authControllerProvider.notifier).forgotPassword(
          email: _email.text.trim(),
          reason: _reason.text.trim().isEmpty ? null : _reason.text.trim(),
        );
    if (!mounted) return;
    setState(() {
      _sending = false;
      final errorMessage = res.fold(
        (_) => null,
        (err) => err.isSmtpNotConfigured
            ? 'Maaf, layanan reset password sedang tidak tersedia. '
                'Silakan hubungi dukungan pelanggan untuk bantuan.'
            : err.message,
      );
      if (errorMessage != null) {
        _done = errorMessage;
        _isError = true;
      } else {
        _done = 'Kami mengirim tautan reset ke email Anda. Cek inbox/spam.';
        _isError = false;
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Scaffold(
      backgroundColor: isp.background,
      appBar: AppBar(
        backgroundColor: isp.background,
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.pop(),
        ),
        title: Text(
          l10n.forgotPassword,
          style: const TextStyle(fontWeight: FontWeight.w800, fontSize: 18),
        ),
        centerTitle: false,
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.symmetric(horizontal: 28, vertical: 32),
          child: Form(
            key: _form,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // ─── Subtitle ───
                Text(
                  'Masukkan email terdaftar. Kami akan kirim kode verifikasi.',
                  style: TextStyle(
                    fontSize: 13,
                    color: isp.textMuted,
                    height: 1.5,
                  ),
                ),
                const SizedBox(height: 28),
                // ─── Label + input ───
                _NeubrutalistLabel(text: l10n.email),
                const SizedBox(height: 8),
                _NeubrutalistInput(
                  controller: _email,
                  hintText: 'nama@email.com',
                  textInputAction: TextInputAction.next,
                  validate: Validators.email,
                ),
                const SizedBox(height: 16),
                _NeubrutalistLabel(text: l10n.reasonOptional),
                const SizedBox(height: 8),
                _NeubrutalistInput(
                  controller: _reason,
                  hintText: l10n.reasonHint,
                  textInputAction: TextInputAction.done,
                  maxLines: 3,
                ),
                // ─── Result banner ───
                if (_done != null) ...[
                  const SizedBox(height: 16),
                  Container(
                    padding: const EdgeInsets.all(14),
                    decoration: BoxDecoration(
                      color: (_isError ? isp.danger : isp.success).withOpacity(0.1),
                      borderRadius: BorderRadius.circular(IspRadii.md),
                      border: Border.all(
                        width: 1.5,
                        color: (_isError ? isp.danger : isp.success).withOpacity(0.2),
                      ),
                    ),
                    child: Row(
                      children: [
                        Icon(
                          _isError ? Icons.warning_amber_rounded : Icons.check_circle_outline,
                          color: _isError ? isp.danger : isp.success,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            _done!,
                            style: TextStyle(
                              color: _isError ? isp.textPrimary : isp.success,
                              fontSize: 13,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
                const SizedBox(height: 28),
                // ─── Send code button ───
                _NeubrutalistAccentButton(
                  label: 'Kirim Kode',
                  loading: _sending,
                  onTap: _submit,
                ),
                const SizedBox(height: 16),
                // ─── Back to login ───
                TextButton(
                  onPressed: () => context.pop(),
                  style: TextButton.styleFrom(
                    foregroundColor: isp.textSecondary,
                  ),
                  child: Text(l10n.backToLogin),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// ─── Neubrutalist widget helpers ────────────────────────────────

class _NeubrutalistLabel extends StatelessWidget {
  const _NeubrutalistLabel({required this.text});
  final String text;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Text(
      text,
      style: TextStyle(
        fontSize: 10,
        fontWeight: FontWeight.w700,
        letterSpacing: 1.5,
        color: isp.textMuted,
      ),
    );
  }
}

class _NeubrutalistInput extends StatelessWidget {
  const _NeubrutalistInput({
    required this.controller,
    required this.hintText,
    this.textInputAction,
    this.validate,
    this.maxLines = 1,
  });
  final TextEditingController controller;
  final String hintText;
  final TextInputAction? textInputAction;
  final String? Function(String?)? validate;
  final int maxLines;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return TextFormField(
      controller: controller,
      keyboardType: TextInputType.emailAddress,
      textInputAction: textInputAction,
      maxLines: maxLines,
      style: TextStyle(color: isp.textPrimary, fontSize: 14),
      decoration: InputDecoration(
        hintText: hintText,
        hintStyle: TextStyle(color: isp.textMuted),
        filled: true,
        fillColor: isp.surface,
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(IspRadii.md),
          borderSide: BorderSide(width: 1.5, color: isp.border),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(IspRadii.md),
          borderSide: BorderSide(width: 1.5, color: isp.border),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(IspRadii.md),
          borderSide: BorderSide(width: 1.5, color: isp.accent),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(IspRadii.md),
          borderSide: BorderSide(width: 1.5, color: isp.danger),
        ),
        focusedErrorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(IspRadii.md),
          borderSide: BorderSide(width: 1.5, color: isp.danger),
        ),
      ),
      validator: validate,
    );
  }
}

class _NeubrutalistAccentButton extends StatelessWidget {
  const _NeubrutalistAccentButton({
    required this.label,
    required this.loading,
    required this.onTap,
  });
  final String label;
  final bool loading;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return GestureDetector(
      onTap: loading ? null : onTap,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(vertical: 14),
        decoration: BoxDecoration(
          color: isp.accent,
          border: Border.all(width: 1.5, color: isp.accent),
          borderRadius: BorderRadius.circular(IspRadii.md),
          boxShadow: [
            BoxShadow(
              offset: const Offset(3, 3),
              blurRadius: 0,
              color: isp.accent.withOpacity(0.3),
            ),
          ],
        ),
        child: Center(
          child: loading
              ? SizedBox(
                  width: 18,
                  height: 18,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    valueColor: AlwaysStoppedAnimation<Color>(isp.textInverse),
                  ),
                )
              : Text(
                  label,
                  style: const TextStyle(
                    color: Colors.white,
                    fontSize: 14,
                    fontWeight: FontWeight.w700,
                  ),
                ),
        ),
      ),
    );
  }
}
