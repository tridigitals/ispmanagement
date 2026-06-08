import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:otp/otp.dart';
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
    final res = await ref.read(authControllerProvider.notifier).start2faEnroll();
    if (!mounted) return;
    setState(() {
      _loading = false;
      _enrollment = res.fold((d) => d, (_) => null);
    });
  }

  String get _currentCode {
    if (_enrollment == null) return '------';
    return OTP.generateTOTPCodeString(
      _enrollment!.secret,
      DateTime.now().millisecondsSinceEpoch,
      interval: _enrollment!.periodSeconds,
      algorithm: Algorithm.SHA1,
      isGoogle: true,
    );
  }

  Future<void> _confirm() async {
    if (!_form.currentState!.validate()) return;
    if (_enrollment == null) return;
    setState(() => _loading = true);
    final res = await ref
        .read(authControllerProvider.notifier)
        .confirm2faEnroll(
          enrollmentId: _enrollment!.enrollmentId,
          code: _code.text.trim(),
        );
    if (!mounted) return;
    setState(() => _loading = false);
    res.fold(
      (_) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(AppLocalizations.of(context)!.twoFaEnabled),
          ),
        );
        context.pop();
      },
      (err) => ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(err.message),
          backgroundColor: IspColors.danger,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final enrollment = _enrollment;
    return Scaffold(
      appBar: AppBar(title: Text(l10n.enable2fa)),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(IspSpacing.lg),
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
                      const SizedBox(height: IspSpacing.lg),
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
                        style:
                            const TextStyle(color: IspColors.textTertiary),
                        textAlign: TextAlign.center,
                      ),
                      const SizedBox(height: IspSpacing.xxl),
                      Card(
                        child: Padding(
                          padding: const EdgeInsets.all(IspSpacing.lg),
                          child: Column(
                            children: [
                              Container(
                                padding: const EdgeInsets.all(IspSpacing.md),
                                decoration: BoxDecoration(
                                  color: Colors.white,
                                  borderRadius:
                                      BorderRadius.circular(IspRadii.md),
                                ),
                                child: QrImageView(
                                  data: enrollment!.otpAuthUri,
                                  version: QrVersions.auto,
                                  size: 200,
                                ),
                              ),
                              const SizedBox(height: IspSpacing.md),
                              SelectableText(
                                enrollment!.secret,
                                style: const TextStyle(
                                  fontFamily: 'monospace',
                                  fontSize: 14,
                                  letterSpacing: 2,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                      const SizedBox(height: IspSpacing.lg),
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
                        decoration: const InputDecoration(
                          labelText: 'Kode 6 digit',
                          counterText: '',
                          hintText: '123 456',
                        ),
                        inputFormatters: [
                          FilteringTextInputFormatter.digitsOnly,
                        ],
                        validator: Validators.otp,
                      ),
                      const SizedBox(height: IspSpacing.xl),
                      IspPrimaryButton(
                        label: l10n.confirmEnable,
                        loading: _loading,
                        onPressed: _confirm,
                      ),
                      const SizedBox(height: IspSpacing.md),
                      TextButton(
                        onPressed: () => context.pop(),
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
