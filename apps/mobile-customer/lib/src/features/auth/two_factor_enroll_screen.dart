import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:qr_flutter/qr_flutter.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../utils/form_validators.dart';

class TwoFactorEnrollScreen extends ConsumerStatefulWidget {
  const TwoFactorEnrollScreen({super.key});
  @override
  ConsumerState<TwoFactorEnrollScreen> createState() => _State();
}

class _State extends ConsumerState<TwoFactorEnrollScreen> {
  late final IspThemeColors isp;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }

  final _form = GlobalKey<FormState>();
  final _code = TextEditingController();
  bool _loading = false;
  TwoFactorEnrollment? _enrollment;

  @override
  void initState() {
    super.initState();
    _enroll();
  }

  @override
  void dispose() {
    _code.dispose();
    super.dispose();
  }

  Future<void> _enroll() async {
    setState(() => _loading = true);
    final res =
        await ref.read(authControllerProvider.notifier).start2faEnroll();
    if (!mounted) return;
    setState(() {
      _loading = false;
      _enrollment = res.fold((d) => d, (_) => null);
    });
  }



  Future<void> _confirm() async {
    if (!_form.currentState!.validate()) return;
    if (_enrollment == null) return;
    setState(() => _loading = true);
    final res =
        await ref.read(authControllerProvider.notifier).confirm2faEnroll(
              enrollmentId: _enrollment!.enrollmentId,
              code: _code.text.trim(),
            );
    if (!mounted) return;
    setState(() => _loading = false);
    res.fold(
      (_) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(AppLocalizations.of(context).twoFaEnabled),
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

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final enrollment = _enrollment;

    return Scaffold(
      backgroundColor: isp.background,
      appBar: AppBar(
        backgroundColor: isp.background,
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.pop(),
        ),
        title: Text(
          l10n.enable2fa,
          style: const TextStyle(fontWeight: FontWeight.w800, fontSize: 18),
        ),
        centerTitle: false,
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.symmetric(horizontal: 28, vertical: 32),
          child: enrollment == null && _loading
              ? const Padding(
                  padding: EdgeInsets.all(48),
                  child: Center(child: CircularProgressIndicator()),
                )
              : Form(
                  key: _form,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      // ─── Headline ───
                      Text(
                        l10n.twoFaHeadline,
                        style: const TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.w700,
                        ),
                        textAlign: TextAlign.center,
                      ),
                      const SizedBox(height: 6),
                      Text(
                        l10n.twoFaSub,
                        style: TextStyle(color: isp.textMuted),
                        textAlign: TextAlign.center,
                      ),
                      const SizedBox(height: 28),
                      // ─── QR Code Card (neubrutalist) ───
                      Container(
                        padding: const EdgeInsets.all(20),
                        decoration: BoxDecoration(
                          color: isp.surface,
                          border: Border.all(width: 1.5, color: isp.border),
                          borderRadius: BorderRadius.circular(24),
                          boxShadow: [
                            BoxShadow(
                              offset: const Offset(3, 3),
                              blurRadius: 0,
                              color: isp.border.withOpacity(0.5),
                            ),
                          ],
                        ),
                        child: Column(
                          children: [
                            Container(
                              padding: const EdgeInsets.all(IspSpacing.md),
                              decoration: BoxDecoration(
                                color: Colors.white,
                                borderRadius: BorderRadius.circular(IspRadii.md),
                              ),
                              child: QrImageView(
                                data: enrollment!.otpAuthUri,
                                version: QrVersions.auto,
                                size: 180,
                              ),
                            ),
                            const SizedBox(height: 16),
                            // ─── Manual code + copy ───
                            Container(
                              padding: const EdgeInsets.symmetric(
                                  horizontal: 16, vertical: 12),
                              decoration: BoxDecoration(
                                color: isp.surface,
                                borderRadius: BorderRadius.circular(IspRadii.md),
                                border: Border.all(
                                    width: 1.5, color: isp.border),
                                boxShadow: [
                                  BoxShadow(
                                    offset: const Offset(3, 3),
                                    blurRadius: 0,
                                    color: isp.border.withOpacity(0.5),
                                  ),
                                ],
                              ),
                              child: Row(
                                children: [
                                  Expanded(
                                    child: SelectableText(
                                      enrollment.secret,
                                      style: const TextStyle(
                                        fontFamily: 'monospace',
                                        fontSize: 14,
                                        letterSpacing: 2,
                                      ),
                                    ),
                                  ),
                                  IconButton(
                                    icon: Icon(Icons.copy,
                                        size: 18, color: isp.accentLight),
                                    onPressed: () {
                                      Clipboard.setData(ClipboardData(
                                          text: enrollment.secret));
                                      ScaffoldMessenger.of(context)
                                          .showSnackBar(
                                        const SnackBar(
                                          content: Text('Kode disalin'),
                                          duration: Duration(seconds: 1),
                                        ),
                                      );
                                    },
                                    visualDensity: VisualDensity.compact,
                                  ),
                                ],
                              ),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(height: 24),
                      // ─── Verify code input ───
                      _NeubrutalistLabel(text: 'Kode 6 digit'),
                      const SizedBox(height: 8),
                      TextFormField(
                        controller: _code,
                        keyboardType: TextInputType.number,
                        maxLength: 6,
                        textAlign: TextAlign.center,
                        style: const TextStyle(
                          fontSize: 28,
                          fontWeight: FontWeight.w800,
                          letterSpacing: 8,
                        ),
                        decoration: NbStyle.inputField(context, hint: '123 456'),
                        inputFormatters: [
                          FilteringTextInputFormatter.digitsOnly,
                        ],
                        validator: Validators.otp,
                      ),
                      const SizedBox(height: 28),
                      // ─── Confirm button ───
                      _NeubrutalistAccentButton(
                        label: l10n.confirmEnable,
                        loading: _loading,
                        onTap: _confirm,
                      ),
                      const SizedBox(height: 16),
                      TextButton(
                        onPressed: () => context.pop(),
                        style: TextButton.styleFrom(
                          foregroundColor: isp.textSecondary,
                        ),
                        child: Text(l10n.cancel),
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
        decoration: NbStyle.card(context), // neubrutalist
                        // color: isp.surface,
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
