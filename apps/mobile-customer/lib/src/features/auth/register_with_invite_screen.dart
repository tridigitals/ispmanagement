import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../utils/form_validators.dart';

class RegisterWithInviteScreen extends ConsumerStatefulWidget {
  const RegisterWithInviteScreen({super.key});
  @override
  ConsumerState<RegisterWithInviteScreen> createState() => _State();
}

class _State extends ConsumerState<RegisterWithInviteScreen> {
  final _form = GlobalKey<FormState>();
  final _invite = TextEditingController();
  final _name = TextEditingController();
  final _email = TextEditingController();
  final _phone = TextEditingController();
  final _password = TextEditingController();
  final _confirm = TextEditingController();
  bool _loading = false;
  InviteValidation? _inviteInfo;

  @override
  void dispose() {
    _invite.dispose();
    _name.dispose();
    _email.dispose();
    _phone.dispose();
    _password.dispose();
    _confirm.dispose();
    super.dispose();
  }

  Future<void> _validateInvite() async {
    if (Validators.inviteCode(_invite.text) != null) {
      _form.currentState?.validate();
      return;
    }
    setState(() {
      _loading = true;
      _inviteInfo = null;
    });
    final res = await ref
        .read(authControllerProvider.notifier)
        .validateInvite(code: _invite.text.trim().toUpperCase());
    if (!mounted) return;
    setState(() {
      _loading = false;
      _inviteInfo = res.fold((d) => d, (_) => null);
    });
  }

  Future<void> _submit() async {
    if (!_form.currentState!.validate()) return;
    if (_inviteInfo == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(AppLocalizations.of(context)!.inviteValidateFirst),
        ),
      );
      return;
    }
    setState(() => _loading = true);
    final res = await ref
        .read(authControllerProvider.notifier)
        .acceptInvite(
          code: _invite.text.trim().toUpperCase(),
          name: _name.text.trim(),
          email: _email.text.trim(),
          phone: _phone.text.trim(),
          password: _password.text,
        );
    if (!mounted) return;
    setState(() => _loading = false);
    res.fold(
      (_) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(AppLocalizations.of(context)!.registerSuccess),
          ),
        );
        context.go('/');
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
    return Scaffold(
      appBar: AppBar(title: Text(l10n.createAccount)),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(IspSpacing.lg),
          child: Form(
            key: _form,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const SizedBox(height: IspSpacing.lg),
                Text(
                  l10n.registerHeadline,
                  style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w700,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 6),
                Text(
                  l10n.registerSub,
                  style:
                      const TextStyle(color: IspColors.textTertiary),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: IspSpacing.xxl),
                TextFormField(
                  controller: _invite,
                  textCapitalization: TextCapitalization.characters,
                  decoration: InputDecoration(
                    labelText: l10n.inviteCode,
                    hintText: 'ISP-2024-ABCDE',
                    prefixIcon: const Icon(Icons.card_giftcard),
                    suffixIcon: IconButton(
                      icon: const Icon(Icons.search),
                      onPressed: _loading ? null : _validateInvite,
                    ),
                  ),
                  validator: Validators.inviteCode,
                  onChanged: (_) {
                    if (_inviteInfo != null) {
                      setState(() => _inviteInfo = null);
                    }
                  },
                ),
                if (_inviteInfo != null) ...[
                  const SizedBox(height: 8),
                  Container(
                    padding: const EdgeInsets.all(IspSpacing.md),
                    decoration: BoxDecoration(
                      color: IspColors.success.withOpacity(0.1),
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
                            '${_inviteInfo!.customerName} — Paket ${_inviteInfo!.packageName}',
                            style: const TextStyle(color: IspColors.success),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
                const SizedBox(height: IspSpacing.lg),
                TextFormField(
                  controller: _name,
                  textInputAction: TextInputAction.next,
                  decoration: InputDecoration(
                    labelText: l10n.fullName,
                    prefixIcon: const Icon(Icons.person_outline),
                  ),
                  validator: (v) =>
                      Validators.required(v, label: l10n.fullName),
                ),
                const SizedBox(height: IspSpacing.md),
                TextFormField(
                  controller: _email,
                  keyboardType: TextInputType.emailAddress,
                  textInputAction: TextInputAction.next,
                  decoration: InputDecoration(
                    labelText: l10n.email,
                    prefixIcon: const Icon(Icons.email_outlined),
                  ),
                  validator: Validators.email,
                ),
                const SizedBox(height: IspSpacing.md),
                TextFormField(
                  controller: _phone,
                  keyboardType: TextInputType.phone,
                  textInputAction: TextInputAction.next,
                  decoration: InputDecoration(
                    labelText: l10n.phone,
                    prefixIcon: const Icon(Icons.phone_outlined),
                  ),
                  validator: Validators.phone,
                ),
                const SizedBox(height: IspSpacing.md),
                TextFormField(
                  controller: _password,
                  obscureText: true,
                  textInputAction: TextInputAction.next,
                  decoration: InputDecoration(
                    labelText: l10n.password,
                    prefixIcon: const Icon(Icons.lock_outline),
                    helperText: l10n.passwordRule,
                  ),
                  validator: Validators.password,
                ),
                const SizedBox(height: IspSpacing.md),
                TextFormField(
                  controller: _confirm,
                  obscureText: true,
                  decoration: InputDecoration(
                    labelText: l10n.confirmPassword,
                    prefixIcon: const Icon(Icons.lock_outline),
                  ),
                  validator: (v) => Validators.matches(
                    v,
                    _password.text,
                    message: l10n.passwordMismatch,
                  ),
                ),
                const SizedBox(height: IspSpacing.xxl),
                IspPrimaryButton(
                  label: l10n.createAccount,
                  loading: _loading,
                  onPressed: _inviteInfo != null ? _submit : null,
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
