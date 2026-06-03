import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../../l10n/app_localizations.dart';
import '../../../services/auth_providers.dart';
import '../../../utils/form_validators.dart';

class OtpLoginScreen extends ConsumerStatefulWidget {
  const OtpLoginScreen({super.key});
  @override
  ConsumerState<OtpLoginScreen> createState() => _State();
}

class _State extends ConsumerState<OtpLoginScreen> {
  final _form = GlobalKey<FormState>();
  final _phone = TextEditingController();
  bool _sending = false;
  int _cooldown = 0;

  @override
  void dispose() {
    _phone.dispose();
    super.dispose();
  }

  Future<void> _sendOtp() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _sending = true);
    final res = await ref
        .read(authControllerProvider.notifier)
        .requestOtp(phone: _phone.text.trim());
    if (!mounted) return;
    setState(() {
      _sending = false;
    });
    res.fold(
      (_) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(AppLocalizations.of(context)!.otpSent)),
        );
        context.push(
          '/login/otp/verify',
          extra: _phone.text.trim(),
        );
        _startCooldown();
      },
      (err) => ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(err.userMessage),
          backgroundColor: IspColors.danger,
        ),
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
      appBar: AppBar(title: Text(l10n.loginWithOtp)),
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
                  Icons.sms_outlined,
                  size: 64,
                  color: IspColors.primary,
                ),
                const SizedBox(height: IspSpacing.lg),
                Text(
                  l10n.otpLoginHeadline,
                  style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 6),
                Text(
                  l10n.otpLoginSub,
                  style: const TextStyle(color: IspColors.textTertiary),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: IspSpacing.xxl),
                TextFormField(
                  controller: _phone,
                  keyboardType: TextInputType.phone,
                  textInputAction: TextInputAction.done,
                  autofillHints: const [AutofillHints.telephoneNumber],
                  decoration: const InputDecoration(
                    labelText: 'No. HP',
                    hintText: '081234567890',
                    prefixIcon: Icon(Icons.phone_outlined),
                  ),
                  validator: Validators.phone,
                  onFieldSubmitted: (_) => _sendOtp(),
                ),
                const SizedBox(height: IspSpacing.xl),
                IspPrimaryButton(
                  label: l10n.sendOtp,
                  loading: _sending,
                  onPressed: _cooldown > 0 ? null : _sendOtp,
                ),
                const SizedBox(height: IspSpacing.md),
                TextButton(
                  onPressed: () => context.pop(),
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
