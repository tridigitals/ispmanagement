import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:local_auth/local_auth.dart';

import 'package:api_client/api_client.dart';
import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/app_config.dart';
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
  late final IspThemeColors isp;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }
  final _passwordCtrl = TextEditingController();
  bool _obscure = true;
  bool _show2fa = false;
  bool _show2faSetup = false;
  String? _setupSecret;
  String? _setupQr;
  bool _setupLoading = false;
  bool _setupEmailSent = false;
  String? _setupMethod = 'totp';
  String? _tempToken;
  final _codeCtrl = TextEditingController();
  final _setupCodeCtrl = TextEditingController();
  bool _biometricAttempted = false;
  bool _biometricLoading = false;

  @override
  void dispose() {
    _emailCtrl.dispose();
    _passwordCtrl.dispose();
    _codeCtrl.dispose();
    _setupCodeCtrl.dispose();
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
        if (data.requires2faSetup && data.tempToken != null) {
          setState(() {
            _show2faSetup = true;
            _tempToken = data.tempToken;
          });
          _start2faSetup();
        } else if (data.requires2fa && data.tempToken != null) {
          setState(() {
            _show2fa = true;
            _tempToken = data.tempToken;
          });
        } else {
          if (mounted) context.go('/');
        }
      case Failure(:final exception):
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(exception.message)),
        );
    }
  }

  Future<void> _start2faSetup() async {
    setState(() => _setupLoading = true);
    try {
      if (_setupMethod == 'email') {
        final dio = ref.read(dioProvider);
        await dio.post('/api/auth/2fa/temp/email/enable-request',
            data: {'tempToken': _tempToken});
        setState(() => _setupEmailSent = true);
      } else {
        final dio = ref.read(dioProvider);
        final res = await dio.post<Map<String, dynamic>>(
          '/api/auth/2fa/temp/enable',
          data: {'tempToken': _tempToken},
        );
        final data = res.data ?? const {};
        setState(() {
          _setupSecret = (data['secret'] as String?) ?? '';
          _setupQr = (data['qr'] as String?) ?? '';
        });
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(e.toString())),
        );
      }
    } finally {
      if (mounted) setState(() => _setupLoading = false);
    }
  }

  Future<void> _switchSetupMethod(String method) async {
    if (_setupMethod == method) return;
    setState(() {
      _setupMethod = method;
      _setupSecret = null;
      _setupQr = null;
      _setupEmailSent = false;
      _setupCodeCtrl.clear();
    });
    await _start2faSetup();
  }

  Future<void> _submit2faSetup() async {
    if (_tempToken == null) return;
    if (_setupCodeCtrl.text.length < 4) return;
    setState(() => _setupLoading = true);
    try {
      final dio = ref.read(dioProvider);
      Map<String, dynamic> payload;
      String endpoint;
      if (_setupMethod == 'email') {
        endpoint = '/api/auth/2fa/temp/email/enable-verify';
        payload = {'tempToken': _tempToken, 'code': _setupCodeCtrl.text.trim()};
      } else {
        endpoint = '/api/auth/2fa/temp/verify-setup';
        payload = {
          'tempToken': _tempToken,
          'secret': _setupSecret,
          'code': _setupCodeCtrl.text.trim(),
        };
      }
      final res = await dio.post<Map<String, dynamic>>(endpoint, data: payload);
      final authResponse = AuthResponse.fromJson(res.data ?? const {});
      if (mounted) {
        await ref.read(authControllerProvider.notifier).apply(authResponse);
        context.go('/');
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(e.toString()),
            backgroundColor: isp.danger,
          ),
        );
      }
    } finally {
      if (mounted) setState(() => _setupLoading = false);
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
                  if (_show2faSetup) ...[
                    _build2FASetupUI(),
                  ] else if (_show2fa) ...[
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
                  ] else ...[
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
                      Padding(
                        padding: EdgeInsets.symmetric(vertical: 12),
                        child: Center(
                          child: Column(
                            children: [
                              Icon(
                                Icons.fingerprint,
                                size: 48,
                                color: isp.accent,
                              ),
                              SizedBox(height: 8),
                              Text(
                                'Verifikasi sidik jari...',
                                style: TextStyle(
                                  color: isp.textMuted,
                                  fontSize: 13,
                                ),
                              ),
                            ],
                          ),
                        ),
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

  Widget _build2FASetupUI() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          'Security Setup Required',
          style: Theme.of(context).textTheme.titleMedium,
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: 4),
        Text(
          'Your organization requires 2FA. Please set it up.',
          style: TextStyle(color: isp.textMuted),
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: 16),
        if (_setupLoading)
          const Center(
            child: Padding(
              padding: EdgeInsets.all(32),
              child: CircularProgressIndicator(),
            ),
          )
        else ...[
          // Method tabs
          Row(
            children: [
              Expanded(
                child: _SetupMethodTab(
                  icon: Icons.smartphone,
                  label: 'Authenticator',
                  selected: _setupMethod == 'totp',
                  onTap: () => _switchSetupMethod('totp'),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: _SetupMethodTab(
                  icon: Icons.email_outlined,
                  label: 'Email',
                  selected: _setupMethod == 'email',
                  onTap: () => _switchSetupMethod('email'),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          if (_setupMethod == 'totp') ...[
            if (_setupQr != null) ...[
              Center(
                child: Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: Colors.white,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Image.memory(
                    const Base64Decoder().convert(_setupQr!),
                    width: 180,
                    height: 180,
                  ),
                ),
              ),
              const SizedBox(height: 8),
              Text(
                'Key: ${_setupSecret ?? ""}',
                style: TextStyle(
                  fontSize: 11,
                  fontFamily: 'monospace',
                  color: isp.textMuted,
                ),
                textAlign: TextAlign.center,
              ),
            ],
            const SizedBox(height: 16),
            TextField(
              controller: _setupCodeCtrl,
              maxLength: 6,
              keyboardType: TextInputType.number,
              textAlign: TextAlign.center,
              style: const TextStyle(
                fontSize: 24,
                letterSpacing: 8,
                fontWeight: FontWeight.w600,
              ),
              decoration: const InputDecoration(
                hintText: '••••••',
                counterText: '',
              ),
            ),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _setupLoading ? null : _submit2faSetup,
              child: const Text('Activate & Login'),
            ),
          ] else ...[
            if (_setupEmailSent)
              Container(
                padding: const EdgeInsets.all(12),
                margin: const EdgeInsets.only(bottom: 16),
                decoration: BoxDecoration(
                  color: Color(0x1F10B981),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    Icon(Icons.check_circle,
                        color: isp.success, size: 18),
                    SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        'Verification code sent to your email.',
                        style: TextStyle(
                            color: isp.success, fontSize: 13),
                      ),
                    ),
                  ],
                ),
              ),
            TextField(
              controller: _setupCodeCtrl,
              maxLength: 6,
              keyboardType: TextInputType.number,
              textAlign: TextAlign.center,
              style: const TextStyle(
                fontSize: 24,
                letterSpacing: 8,
                fontWeight: FontWeight.w600,
              ),
              decoration: const InputDecoration(
                hintText: '••••••',
                counterText: '',
              ),
            ),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _setupLoading ? null : _submit2faSetup,
              child: const Text('Verify & Login'),
            ),
          ],
        ],
      ],
    );
  }
}

class _SetupMethodTab extends StatelessWidget {
  const _SetupMethodTab({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 10),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: selected ? isp.accent : isp.border,
            width: selected ? 2 : 1,
          ),
          color: selected ? isp.accentSurface : null,
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              icon,
              size: 16,
              color: selected ? isp.accent : isp.textMuted,
            ),
            const SizedBox(width: 6),
            Text(
              label,
              style: TextStyle(
                fontSize: 13,
                fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                color: selected ? isp.accent : isp.textSecondary,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _BrandHeader extends StatelessWidget {
  const _BrandHeader();
  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Column(
      children: [
        Container(
          padding: const EdgeInsets.all(20),
          decoration: BoxDecoration(
            color: isp.accentSurface,
            shape: BoxShape.circle,
          ),
          child: Icon(Icons.wifi_tethering,
              size: 40, color: isp.accent),
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
                color: isp.textMuted,
              ),
        ),
      ],
    );
  }
}
