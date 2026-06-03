import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../../l10n/app_localizations.dart';
import '../../../services/auth_providers.dart';
import '../../../utils/form_validators.dart';

class OtpVerifyScreen extends ConsumerStatefulWidget {
  const OtpVerifyScreen({super.key, this.phone});
  final String? phone;
  @override
  ConsumerState<OtpVerifyScreen> createState() => _State();
}

class _State extends ConsumerState<OtpVerifyScreen> {
  final _form = GlobalKey<FormState>();
  final _code = TextEditingController();
  bool _verifying = false;
  int _cooldown = 60;
  late final String _phone;

  @override
  void initState() {
    super.initState();
    _phone = widget.phone ?? '';
    _startCooldown();
  }

  @override
  void dispose() {
    _code.dispose();
    super.dispose();
  }

  Future<void> _verify() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _verifying = true);
    final res = await ref
        .read(authControllerProvider.notifier)
        .loginWithOtp(phone: _phone, code: _code.text.trim());
    if (!mounted) return;
    setState(() => _verifying = false);
    res.fold(
      (_) => context.go('/'),
      (err) => ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(err.userMessage),
          backgroundColor: IspColors.danger,
        ),
      ),
    );
  }

  Future<void> _resend() async {
    if (_cooldown > 0) return;
    final res = await ref
        .read(authControllerProvider.notifier)
        .requestOtp(phone: _phone);
    if (!mounted) return;
    res.fold(
      (_) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(AppLocalizations.of(context)!.otpResent)),
        );
        _startCooldown();
      },
      (err) => ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(err.userMessage)),
      ),
    );
  }

  void _startCooldown() {
    setState(() => _cooldown = 60);
    Stream.periodic(const Duration(seconds: 1), (i) => 60 - i - 1)
        .take(60)
        .forEach((s) {
      if (mounted) setState(() => _cooldown = s);
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(title: Text(l10n.verifyOtp)),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(IspSpacing.lg),
          child: Form(
            key: _form,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const SizedBox(height: IspSpacing.xl),
                const Icon(
                  Icons.mark_email_read_outlined,
                  size: 64,
                  color: IspColors.primary,
                ),
                const SizedBox(height: IspSpacing.lg),
                Text(
                  l10n.otpVerifyHeadline,
                  style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 6),
                Text(
                  l10n.otpVerifySub(_phone),
                  style: const TextStyle(color: IspColors.textTertiary),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: IspSpacing.xxl),
                TextFormField(
                  controller: _code,
                  keyboardType: TextInputType.number,
                  textAlign: TextAlign.center,
                  style: const TextStyle(
                    fontSize: 32,
                    fontWeight: FontWeight.w800,
                    letterSpacing: 12,
                  ),
                  maxLength: 6,
                  autofillHints: const [AutofillHints.oneTimeCode],
                  decoration: const InputDecoration(
                    counterText: '',
                    hintText: '••••••',
                  ),
                  inputFormatters: [
                    FilteringTextInputFormatter.digitsOnly,
                  ],
                  validator: Validators.otp,
                  onChanged: (v) {
                    if (v.length == 6) {
                      _verify();
                    }
                  },
                ),
                const SizedBox(height: IspSpacing.lg),
                Center(
                  child: TextButton(
                    onPressed: _cooldown > 0 ? null : _resend,
                    child: Text(
                      _cooldown > 0
                          ? l10n.resendIn(_cooldown)
                          : l10n.resendOtp,
                    ),
                  ),
                ),
                const SizedBox(height: IspSpacing.xl),
                IspPrimaryButton(
                  label: l10n.verify,
                  loading: _verifying,
                  onPressed: _verify,
                ),
                const SizedBox(height: IspSpacing.md),
                TextButton(
                  onPressed: () => context.pop(),
                  child: Text(l10n.back),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
