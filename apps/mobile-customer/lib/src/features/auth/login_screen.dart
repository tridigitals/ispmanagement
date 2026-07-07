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
import '../../services/fcm_service.dart';
import '../../services/missing_providers.dart';
import '../../services/service_providers.dart';

class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});

  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen> {
  final _formKey = GlobalKey<FormState>();
  final _identifierCtrl = TextEditingController();
  final _passwordCtrl = TextEditingController();
  late final IspThemeColors isp;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }

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
  bool _biometricAvailable = false;
  bool _biometricAttempted = false;
  bool _biometricLoading = false;

  @override
  void dispose() {
    _identifierCtrl.dispose();
    _passwordCtrl.dispose();
    _codeCtrl.dispose();
    _setupCodeCtrl.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _tryBiometricLogin());
  }

  Future<void> _tryBiometricLogin() async {
    if (_biometricAttempted) return;
    await ref.read(biometricEnabledProvider.future);
    final biometricEnabled =
        ref.read(biometricEnabledProvider).valueOrNull ?? false;
    if (!biometricEnabled) {
      _biometricAttempted = true;
      return;
    }
    final authSvc = ref.read(authServiceProvider);
    final hasSession = await authSvc.hasSession();
    if (!hasSession) {
      _biometricAttempted = true;
      return;
    }
    final auth = LocalAuthentication();
    final canCheck = await auth.canCheckBiometrics;
    if (!canCheck) {
      _biometricAttempted = true;
      return;
    }
    if (!mounted) return;
    _biometricAttempted = true;
    setState(() {
      _biometricAvailable = true;
      _biometricLoading = true;
    });
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
      final restored =
          await ref.read(authControllerProvider.notifier).bootstrap();
      if (mounted) {
        if (restored) {
          ref.read(fcmServiceProvider).clearPendingAction();
          context.go('/loading');
        } else {
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
      identifier: _identifierCtrl.text.trim(),
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
          if (mounted) {
            ref.read(fcmServiceProvider).clearPendingAction();
            context.go('/loading');
          }
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
      final applied =
          await ref.read(authControllerProvider.notifier).apply(authResponse);
      if (applied is Failure<bool>) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(applied.exception.message),
              backgroundColor: isp.danger,
            ),
          );
        }
        return;
      }
      if (mounted) {
        ref.read(fcmServiceProvider).clearPendingAction();
        context.go('/loading');
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
        final applied =
            await ref.read(authControllerProvider.notifier).apply(data);
        if (applied is Failure<bool>) {
          if (mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text(applied.exception.message),
                backgroundColor: isp.danger,
              ),
            );
          }
          return;
        }
        if (mounted) {
          ref.read(fcmServiceProvider).clearPendingAction();
          context.go('/loading');
        }
      case Failure(:final exception):
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(exception.message)),
        );
    }
  }

  // ─── Neubrutalist decoration helpers ──────────────────────────



  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final loading = ref.watch(authControllerProvider).isLoading;
    return Scaffold(
      backgroundColor: isp.background,
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.symmetric(horizontal: 28, vertical: 48),
            child: Form(
              key: _formKey,
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (_show2faSetup)
                    _build2FASetupUI()
                  else if (_show2fa)
                    _build2FAInlineUI(l10n)
                  else ...[
                    // ─── Solid purple square logo (no gradient) ───
                    _BrandLogo(isp: isp),
                    const SizedBox(height: 36),
                    // ─── Title ───
                    Text(
                      l10n.login,
                      style: const TextStyle(
                        fontSize: 30,
                        fontWeight: FontWeight.w900,
                        letterSpacing: -1,
                        color: Color(0xFFF0F0F5),
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      'Kelola layanan internet Anda.',
                      style: TextStyle(
                        fontSize: 14,
                        color: isp.textMuted,
                      ),
                    ),
                    const SizedBox(height: 36),
                    // ─── Email / Phone label + input ───
                    _NeubrutalistLabel(text: 'Email atau Nomor HP'),
                    const SizedBox(height: 8),
                    _NeubrutalistInput(
                      controller: _identifierCtrl,
                      hintText: 'nama@email.com atau 08xxx',
                      textInputAction: TextInputAction.next,
                      validate: (v) =>
                          (v == null || v.isEmpty) ? 'Wajib diisi' : null,
                    ),
                    const SizedBox(height: 16),
                    // ─── Password label + input ───
                    _NeubrutalistLabel(text: l10n.password),
                    const SizedBox(height: 8),
                    _NeubrutalistPasswordInput(
                      controller: _passwordCtrl,
                      obscure: _obscure,
                      onToggle: () => setState(() => _obscure = !_obscure),
                      onSubmitted: (_) => _submit(),
                      validate: (v) => (v == null || v.length < 6)
                          ? l10n.passwordTooShort
                          : null,
                    ),
                    // ─── Forgot password ───
                    Align(
                      alignment: Alignment.centerRight,
                      child: TextButton(
                        onPressed: () => context.push('/forgot-password'),
                        style: TextButton.styleFrom(
                          foregroundColor: isp.accentLight,
                        ),
                        child: Text(
                          l10n.forgotPassword,
                          style: const TextStyle(
                            fontSize: 13,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(height: 24),
                    // ─── Login button (neubrutalist accent) ───
                    _NeubrutalistAccentButton(
                      label: l10n.login,
                      loading: loading,
                      onTap: _submit,
                    ),
                    const SizedBox(height: 24),
                    // ─── Divider "atau" ───
                    Row(
                      children: [
                        Expanded(
                            child:
                                Divider(color: isp.border, thickness: 1.5)),
                        Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 14),
                          child: Text(
                            'atau',
                            style: TextStyle(
                              fontSize: 12,
                              color: isp.textMuted,
                            ),
                          ),
                        ),
                        Expanded(
                            child:
                                Divider(color: isp.border, thickness: 1.5)),
                      ],
                    ),
                    const SizedBox(height: 24),
                    // ─── Biometric button ───
                    if (_biometricAvailable && !_biometricLoading)
                      _NeubrutalistOutlineButton(
                        icon: Icons.fingerprint,
                        label: l10n.biometric,
                        onTap: () {
                          _biometricAttempted = false;
                          _tryBiometricLogin();
                        },
                      ),
                    if (_biometricLoading)
                      Padding(
                        padding: const EdgeInsets.symmetric(vertical: 12),
                        child: Column(
                          children: [
                            Icon(Icons.fingerprint,
                                size: 48, color: isp.accent),
                            const SizedBox(height: 8),
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
                    // ─── NO "Hubungi ISP" link (multi-tenant) ───
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _build2FAInlineUI(AppLocalizations l10n) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          l10n.enter2faCode,
          style: const TextStyle(
            fontSize: 18,
            fontWeight: FontWeight.w700,
          ),
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
            hintStyle: TextStyle(color: isp.textMuted),
            filled: true,
            fillColor: isp.surfaceTertiary,
            border: OutlineInputBorder(
              borderRadius: BorderRadius.circular(IspRadii.md),
              borderSide: BorderSide.none,
            ),
          ),
        ),
        const SizedBox(height: 16),
        _NeubrutalistAccentButton(
          label: l10n.verify,
          loading: ref.watch(authControllerProvider).isLoading,
          onTap: _submit2fa,
        ),
        const SizedBox(height: 8),
        TextButton(
          onPressed: () => setState(() => _show2fa = false),
          child: Text(l10n.back),
        ),
      ],
    );
  }

  Widget _build2FASetupUI() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          'Security Setup Required',
          style: const TextStyle(
            fontSize: 18,
            fontWeight: FontWeight.w700,
          ),
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
              decoration: InputDecoration(
                hintText: '••••••',
                counterText: '',
                hintStyle: TextStyle(color: isp.textMuted),
                filled: true,
                fillColor: isp.surfaceTertiary,
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(IspRadii.md),
                  borderSide: BorderSide.none,
                ),
              ),
            ),
            const SizedBox(height: 16),
            _NeubrutalistAccentButton(
              label: 'Activate & Login',
              loading: _setupLoading,
              onTap: _submit2faSetup,
            ),
          ] else ...[
            if (_setupEmailSent)
              Container(
                padding: const EdgeInsets.all(12),
                margin: const EdgeInsets.only(bottom: 16),
                decoration: BoxDecoration(
                  color: isp.success.withOpacity(0.1),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    Icon(Icons.check_circle,
                        color: isp.success, size: 18),
                    const SizedBox(width: 8),
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
              decoration: InputDecoration(
                hintText: '••••••',
                counterText: '',
                hintStyle: TextStyle(color: isp.textMuted),
                filled: true,
                fillColor: isp.surfaceTertiary,
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(IspRadii.md),
                  borderSide: BorderSide.none,
                ),
              ),
            ),
            const SizedBox(height: 16),
            _NeubrutalistAccentButton(
              label: 'Verify & Login',
              loading: _setupLoading,
              onTap: _submit2faSetup,
            ),
          ],
        ],
      ],
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
    required this.validate,
  });
  final TextEditingController controller;
  final String hintText;
  final TextInputAction? textInputAction;
  final String? Function(String?)? validate;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return TextFormField(
      controller: controller,
      keyboardType: TextInputType.text,
      textInputAction: textInputAction,
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

class _NeubrutalistPasswordInput extends StatelessWidget {
  const _NeubrutalistPasswordInput({
    required this.controller,
    required this.obscure,
    required this.onToggle,
    this.onSubmitted,
    required this.validate,
  });
  final TextEditingController controller;
  final bool obscure;
  final VoidCallback onToggle;
  final ValueChanged<String>? onSubmitted;
  final String? Function(String?)? validate;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return TextFormField(
      controller: controller,
      obscureText: obscure,
      textInputAction: TextInputAction.done,
      onFieldSubmitted: onSubmitted,
      style: TextStyle(color: isp.textPrimary, fontSize: 14),
      decoration: InputDecoration(
        hintText: '••••••••',
        hintStyle: TextStyle(color: isp.textMuted),
        filled: true,
        fillColor: isp.surface,
        contentPadding:
            const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        suffixIcon: IconButton(
          icon: Icon(
            obscure ? Icons.visibility : Icons.visibility_off,
            color: isp.textMuted,
            size: 20,
          ),
          onPressed: onToggle,
        ),
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
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 100),
        transform: (loading || onTap == null)
            ? Matrix4.identity()
            : Matrix4.translationValues(0, 0, 0),
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
                  style: TextStyle(
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

class _NeubrutalistOutlineButton extends StatelessWidget {
  const _NeubrutalistOutlineButton({
    required this.icon,
    required this.label,
    required this.onTap,
  });
  final IconData icon;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return GestureDetector(
      onTap: onTap,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(vertical: 14),
        decoration: BoxDecoration(
          color: isp.surface,
          border: Border.all(width: 1.5, color: isp.border),
          borderRadius: BorderRadius.circular(IspRadii.md),
          boxShadow: [
            BoxShadow(
              offset: const Offset(3, 3),
              blurRadius: 0,
              color: isp.surfaceElevated,
            ),
          ],
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(icon, size: 22, color: isp.accentLight),
            const SizedBox(width: 10),
            Text(
              label,
              style: TextStyle(
                color: isp.textSecondary,
                fontSize: 14,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ─── Solid purple square brand logo (NO gradient) ───────────────

class _BrandLogo extends StatelessWidget {
  const _BrandLogo({required this.isp});
  final IspThemeColors isp;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Container(
          width: 56,
          height: 56,
          decoration: BoxDecoration(
            color: isp.accent, // solid purple, no gradient
            borderRadius: BorderRadius.circular(16),
          ),
          alignment: Alignment.center,
          child: const Text(
            'IS',
            style: TextStyle(
              fontSize: 26,
              fontWeight: FontWeight.w900,
              color: Colors.white,
              letterSpacing: -1,
            ),
          ),
        ),
      ],
    );
  }
}

// ─── Setup method tab ───────────────────────────────────────────

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
    final isp = context.isp;
    return GestureDetector(
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
