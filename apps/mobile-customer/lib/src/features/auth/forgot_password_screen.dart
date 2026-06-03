import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../../l10n/app_localizations.dart';
import '../../../services/auth_providers.dart';
import '../../../utils/form_validators.dart';

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
    final res = await ref
        .read(authControllerProvider.notifier)
        .forgotPassword(
          email: _email.text.trim(),
          reason: _reason.text.trim().isEmpty ? null : _reason.text.trim(),
        );
    if (!mounted) return;
    setState(() {
      _sending = false;
      _done = res.fold(
        (_) => 'Kami mengirim tautan reset ke email Anda. Cek inbox/spam.',
        (err) => err.userMessage,
      );
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(title: Text(l10n.forgotPassword)),
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
                  Icons.lock_reset_outlined,
                  size: 64,
                  color: IspColors.primary,
                ),
                const SizedBox(height: IspSpacing.lg),
                Text(
                  l10n.forgotPasswordHeadline,
                  style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 6),
                Text(
                  l10n.forgotPasswordSub,
                  style: const TextStyle(color: IspColors.textTertiary),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: IspSpacing.xxl),
                TextFormField(
                  controller: _email,
                  keyboardType: TextInputType.emailAddress,
                  textInputAction: TextInputAction.next,
                  autofillHints: const [AutofillHints.email],
                  decoration: InputDecoration(
                    labelText: l10n.email,
                    prefixIcon: const Icon(Icons.email_outlined),
                  ),
                  validator: Validators.email,
                ),
                const SizedBox(height: IspSpacing.md),
                TextFormField(
                  controller: _reason,
                  maxLines: 3,
                  maxLength: 200,
                  decoration: InputDecoration(
                    labelText: l10n.reasonOptional,
                    hintText: l10n.reasonHint,
                    alignLabelWithHint: true,
                  ),
                ),
                if (_done != null) ...[
                  const SizedBox(height: IspSpacing.md),
                  Container(
                    padding: const EdgeInsets.all(IspSpacing.md),
                    decoration: BoxDecoration(
                      color: IspColors.success.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(IspRadii.md),
                    ),
                    child: Row(
                      children: [
                        const Icon(
                          Icons.check_circle_outline,
                          color: IspColors.success,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            _done!,
                            style: const TextStyle(color: IspColors.success),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
                const SizedBox(height: IspSpacing.xl),
                IspPrimaryButton(
                  label: l10n.sendResetLink,
                  loading: _sending,
                  onPressed: _submit,
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
