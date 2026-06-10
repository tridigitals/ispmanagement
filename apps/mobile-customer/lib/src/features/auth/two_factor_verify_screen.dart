import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../utils/form_validators.dart';

/// 2FA verification at login. The login_screen should push this if the
/// server returns `requires_2fa: true` from `/api/auth/login`.
class TwoFactorVerifyScreen extends ConsumerStatefulWidget {
  const TwoFactorVerifyScreen({
    super.key,
    required this.pendingToken,
  });

  /// Opaque token returned by /api/auth/login when 2FA is required.
  final String pendingToken;
  @override
  ConsumerState<TwoFactorVerifyScreen> createState() => _State();
}

class _State extends ConsumerState<TwoFactorVerifyScreen> {
  final _form = GlobalKey<FormState>();
  final _code = TextEditingController();
  bool _verifying = false;
  bool _usingBackup = false;

  @override
  void dispose() {
    _code.dispose();
    super.dispose();
  }

  Future<void> _verify() async {
    if (!_form.currentState!.validate()) return;
    setState(() => _verifying = true);
    final res = await ref.read(authControllerProvider.notifier).verify2fa(
          tempToken: widget.pendingToken,
          code: _code.text.trim(),
        );
    if (!mounted) return;
    setState(() => _verifying = false);
    res.fold(
      (_) => context.go('/'),
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
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.verify2fa)),
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
                  Icons.security_outlined,
                  size: 64,
                  color: IspColors.primary,
                ),
                const SizedBox(height: IspSpacing.lg),
                Text(
                  l10n.verify2faHeadline,
                  style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: IspSpacing.xxl),
                TextFormField(
                  controller: _code,
                  keyboardType:
                      _usingBackup ? TextInputType.text : TextInputType.number,
                  maxLength: _usingBackup ? 12 : 6,
                  textAlign: TextAlign.center,
                  style: const TextStyle(
                    fontSize: 28,
                    fontWeight: FontWeight.w800,
                    letterSpacing: 8,
                  ),
                  decoration: InputDecoration(
                    labelText: _usingBackup ? l10n.backupCode : l10n.otpCode,
                    counterText: '',
                    hintText: _usingBackup ? 'XXXX-XXXX' : '123 456',
                  ),
                  inputFormatters: _usingBackup
                      ? [
                          FilteringTextInputFormatter.allow(
                            RegExp(r'[A-Za-z0-9-]'),
                          ),
                        ]
                      : [FilteringTextInputFormatter.digitsOnly],
                  validator: (v) => _usingBackup
                      ? Validators.required(v, label: l10n.backupCode)
                      : Validators.otp(v),
                ),
                const SizedBox(height: IspSpacing.lg),
                Center(
                  child: TextButton(
                    onPressed: () =>
                        setState(() => _usingBackup = !_usingBackup),
                    child: Text(
                      _usingBackup ? l10n.useAuthenticator : l10n.useBackupCode,
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
