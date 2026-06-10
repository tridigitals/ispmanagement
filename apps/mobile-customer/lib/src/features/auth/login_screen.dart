import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:local_auth/local_auth.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../services/missing_providers.dart';
import '../../services/service_providers.dart';

class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});

  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen> {
  final _formKey = GlobalKey<FormState>();
  final _emailCtrl = TextEditingController();
  final _passwordCtrl = TextEditingController();
  bool _obscure = true;
  bool _show2fa = false;
  String? _tempToken;
  final _codeCtrl = TextEditingController();
  bool _biometricAttempted = false;
  bool _biometricLoading = false;

  @override
  void dispose() {
    _emailCtrl.dispose();
    _passwordCtrl.dispose();
    _codeCtrl.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    // Auto-prompt fingerprint after build completes
    WidgetsBinding.instance.addPostFrameCallback((_) => _tryBiometricLogin());
  }

  Future<void> _tryBiometricLogin() async {
    if (_biometricAttempted) return;
    _biometricAttempted = true;

    // Wait for biometric provider to finish loading
    await ref.read(biometricEnabledProvider.future);

    // Check if biometric is enabled
    final biometricEnabled =
        ref.read(biometricEnabledProvider).valueOrNull ?? false;
    if (!biometricEnabled) return;

    // Check if there's a stored session (token)
    final authSvc = ref.read(authServiceProvider);
    final hasSession = await authSvc.hasSession();
    if (!hasSession) return; // No token = nothing to restore, skip prompt

    // Check device supports biometric
    final auth = LocalAuthentication();
    final canCheck = await auth.canCheckBiometrics;
    if (!canCheck) return;

    if (!mounted) return;

    setState(() => _biometricLoading = true);

    try {
      final ok = await auth.authenticate(
        localizedReason: 'Gunakan fingerprint untuk login',
        options: const AuthenticationOptions(
          stickyAuth: true,
          biometricOnly: true,
        ),
      );

      if (!ok || !mounted) {
        setState(() => _biometricLoading = false);
        return;
      }

      // Fingerprint verified — restore session from stored token
      final restored =
          await ref.read(authControllerProvider.notifier).bootstrap();

      if (mounted) {
        if (restored) {
          context.go('/');
        } else {
          // Token ada tapi /me gagal (expired/network). Minta user login manual.
          setState(() {
            _biometricLoading = false;
            _biometricAttempted = false;
          });
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(
              content: Text(
                'Sesi berakhir. Silakan login ulang dengan email & password.',
              ),
              duration: Duration(seconds: 4),
            ),
          );
        }
      }
    } catch (e) {
      if (mounted) setState(() => _biometricLoading = false);
    }
  }

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;
    final auth = ref.read(authControllerProvider.notifier);
    final res = await auth.login(
      email: _emailCtrl.text.trim(),
      password: _passwordCtrl.text,
    );
    if (!mounted) return;
    switch (res) {
      case Success(:final data):
        if (data.requires2fa && data.tempToken != null) {
          setState(() {
            _show2fa = true;
            _tempToken = data.tempToken;
          });
        } else {
          await ref.read(authControllerProvider.notifier).apply(data);
          if (mounted) context.go('/');
        }
      case Failure(:final exception):
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(exception.message)),
        );
    }
  }

  Future<void> _submit2fa() async {
    if (_tempToken == null) return;
    if (_codeCtrl.text.length < 4) return;
    final res = await ref.read(authControllerProvider.notifier).verify2fa(
          tempToken: _tempToken!,
          code: _codeCtrl.text.trim(),
        );
    if (!mounted) return;
    switch (res) {
      case Success(:final data):
        await ref.read(authControllerProvider.notifier).apply(data);
        if (mounted) context.go('/');
      case Failure(:final exception):
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(exception.message)),
        );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final loading = ref.watch(authControllerProvider).isLoading;
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: Form(
              key: _formKey,
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const _BrandHeader(),
                  const SizedBox(height: 32),
                  if (!_show2fa) ...[
                    TextFormField(
                      controller: _emailCtrl,
                      keyboardType: TextInputType.emailAddress,
                      textInputAction: TextInputAction.next,
                      decoration: InputDecoration(
                        labelText: l10n.email,
                        prefixIcon: const Icon(Icons.alternate_email),
                      ),
                      validator: (v) => (v == null || !v.contains('@'))
                          ? l10n.invalidEmail
                          : null,
                    ),
                    const SizedBox(height: 12),
                    TextFormField(
                      controller: _passwordCtrl,
                      obscureText: _obscure,
                      textInputAction: TextInputAction.done,
                      onFieldSubmitted: (_) => _submit(),
                      decoration: InputDecoration(
                        labelText: l10n.password,
                        prefixIcon: const Icon(Icons.lock_outline),
                        suffixIcon: IconButton(
                          icon: Icon(_obscure
                              ? Icons.visibility
                              : Icons.visibility_off),
                          onPressed: () => setState(() => _obscure = !_obscure),
                        ),
                      ),
                      validator: (v) => (v == null || v.length < 6)
                          ? l10n.passwordTooShort
                          : null,
                    ),
                    const SizedBox(height: 8),
                    Align(
                      alignment: Alignment.centerRight,
                      child: TextButton(
                        onPressed: () => context.push('/forgot-password'),
                        child: Text(l10n.forgotPassword),
                      ),
                    ),
                    const SizedBox(height: 16),
                    ElevatedButton(
                      onPressed: loading ? null : _submit,
                      child: loading
                          ? const SizedBox(
                              width: 18,
                              height: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : Text(l10n.login),
                    ),
                    const SizedBox(height: 12),
                    // Fingerprint button — show if biometric was enabled and session exists
                    if (_biometricAttempted && !_biometricLoading)
                      OutlinedButton.icon(
                        onPressed: () {
                          _biometricAttempted = false;
                          _tryBiometricLogin();
                        },
                        icon: const Icon(Icons.fingerprint, size: 24),
                        label: Text(l10n.biometric),
                      ),
                    if (_biometricLoading)
                      const Padding(
                        padding: EdgeInsets.symmetric(vertical: 12),
                        child: Center(
                          child: Column(
                            children: [
                              Icon(
                                Icons.fingerprint,
                                size: 48,
                                color: IspColors.primary,
                              ),
                              SizedBox(height: 8),
                              Text(
                                'Verifikasi sidik jari...',
                                style: TextStyle(
                                  color: IspColors.textTertiary,
                                  fontSize: 13,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                  ] else ...[
                    Text(
                      l10n.enter2faCode,
                      style: Theme.of(context).textTheme.titleMedium,
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 24),
                    TextField(
                      controller: _codeCtrl,
                      maxLength: 6,
                      keyboardType: TextInputType.number,
                      textAlign: TextAlign.center,
                      style: const TextStyle(
                        fontSize: 24,
                        letterSpacing: 8,
                        fontWeight: FontWeight.w600,
                      ),
                      decoration: InputDecoration(
                        hintText: '••••••',
                        counterText: '',
                      ),
                    ),
                    const SizedBox(height: 16),
                    ElevatedButton(
                      onPressed: loading ? null : _submit2fa,
                      child: Text(l10n.verify),
                    ),
                    TextButton(
                      onPressed: () => setState(() => _show2fa = false),
                      child: Text(l10n.back),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _BrandHeader extends StatelessWidget {
  const _BrandHeader();
  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Container(
          padding: const EdgeInsets.all(20),
          decoration: BoxDecoration(
            color: IspColors.primarySubtle,
            shape: BoxShape.circle,
          ),
          child: const Icon(Icons.wifi_tethering,
              size: 40, color: IspColors.primary),
        ),
        const SizedBox(height: 16),
        Text(
          'ISP Customer',
          style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                fontWeight: FontWeight.w700,
              ),
        ),
        const SizedBox(height: 4),
        Text(
          'Kelola langganan internet Anda',
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                color: IspColors.textTertiary,
              ),
        ),
      ],
    );
  }
}
