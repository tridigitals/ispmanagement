import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../utils/form_validators.dart';

/// 2FA verification at login. 6 individual OTP boxes with focus-jump.
class TwoFactorVerifyScreen extends ConsumerStatefulWidget {
  const TwoFactorVerifyScreen({
    super.key,
    required this.pendingToken,
  });

  final String pendingToken;
  @override
  ConsumerState<TwoFactorVerifyScreen> createState() => _State();
}

class _State extends ConsumerState<TwoFactorVerifyScreen> {
  late final IspThemeColors isp;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    isp = context.isp;
  }

  final _form = GlobalKey<FormState>();
  final _controllers = List.generate(6, (_) => TextEditingController());
  final _focusNodes = List.generate(6, (_) => FocusNode());
  bool _verifying = false;
  bool _usingBackup = false;

  // ─── Countdown timer ───
  static const _expirySeconds = 150; // 2:30
  int _remaining = _expirySeconds;
  Timer? _timer;
  bool _expired = false;

  @override
  void initState() {
    super.initState();
    _startTimer();
  }

  @override
  void dispose() {
    for (final c in _controllers) {
      c.dispose();
    }
    for (final f in _focusNodes) {
      f.dispose();
    }
    _timer?.cancel();
    super.dispose();
  }

  void _startTimer() {
    _timer?.cancel();
    _timer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (!mounted) return;
      setState(() {
        if (_remaining > 0) {
          _remaining--;
        } else {
          _expired = true;
          _timer?.cancel();
        }
      });
    });
  }

  String get _formattedTime {
    final m = _remaining ~/ 60;
    final s = _remaining % 60;
    return '$m:${s.toString().padLeft(2, '0')}';
  }

  void _onDigitChanged(int index, String value) {
    if (value.length == 1 && index < 5) {
      FocusScope.of(context).requestFocus(_focusNodes[index + 1]);
    }
    // Handle backspace: if empty and not first, jump back
    if (value.isEmpty && index > 0) {
      // We check on next frame because the controller still has old value
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_controllers[index].text.isEmpty) {
          FocusScope.of(context).requestFocus(_focusNodes[index - 1]);
        }
      });
    }
  }

  Future<void> _verify() async {
    final code = _controllers.map((c) => c.text).join();
    if (code.length < (_usingBackup ? 4 : 6)) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Kode harus diisi lengkap')),
      );
      return;
    }
    setState(() => _verifying = true);
    final res = await ref.read(authControllerProvider.notifier).verify2fa(
          tempToken: widget.pendingToken,
          code: code,
        );
    if (!mounted) return;
    setState(() => _verifying = false);
    res.fold(
      (_) => context.go('/'),
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

    return Scaffold(
      backgroundColor: isp.background,
      appBar: AppBar(
        backgroundColor: isp.background,
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.pop(),
        ),
        title: Text(
          l10n.verify2fa,
          style: const TextStyle(fontWeight: FontWeight.w800, fontSize: 18),
        ),
        centerTitle: false,
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.symmetric(horizontal: 28, vertical: 32),
          child: Form(
            key: _form,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const SizedBox(height: 24),
                // ─── 2FA icon ───
                Container(
                  width: 64,
                  height: 64,
                  decoration: BoxDecoration(
                    color: isp.accent.withOpacity(0.12),
                    borderRadius: BorderRadius.circular(18),
                  ),
                  child: Icon(Icons.security_outlined,
                      size: 32, color: isp.accentLight),
                ),
                const SizedBox(height: 20),
                // ─── Headline ───
                Text(
                  l10n.verify2faHeadline,
                  style: const TextStyle(
                    fontSize: 22,
                    fontWeight: FontWeight.w800,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 4),
                Text(
                  'Masukkan 6 digit kode dari aplikasi autentikasi.',
                  style: TextStyle(
                    fontSize: 13,
                    color: isp.textMuted,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 32),
                // ─── 6 OTP boxes ───
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: List.generate(6, (i) {
                    return Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 4),
                      child: SizedBox(
                        width: 44,
                        height: 52,
                        child: TextField(
                          controller: _controllers[i],
                          focusNode: _focusNodes[i],
                          keyboardType: TextInputType.number,
                          textAlign: TextAlign.center,
                          maxLength: 1,
                          style: const TextStyle(
                            fontSize: 22,
                            fontWeight: FontWeight.w800,
                            fontFamily: 'monospace',
                          ),
                          decoration: InputDecoration(
                            counterText: '',
                            filled: true,
                            fillColor: isp.surface,
                            border: OutlineInputBorder(
                              borderRadius:
                                  BorderRadius.circular(IspRadii.sm),
                              borderSide: BorderSide(
                                  width: 1.5, color: isp.border),
                            ),
                            enabledBorder: OutlineInputBorder(
                              borderRadius:
                                  BorderRadius.circular(IspRadii.sm),
                              borderSide: BorderSide(
                                  width: 1.5, color: isp.border),
                            ),
                            focusedBorder: OutlineInputBorder(
                              borderRadius:
                                  BorderRadius.circular(IspRadii.sm),
                              borderSide: BorderSide(
                                  width: 1.5, color: isp.accent),
                            ),
                          ),
                          inputFormatters: [
                            FilteringTextInputFormatter.digitsOnly,
                          ],
                          onChanged: (v) => _onDigitChanged(i, v),
                        ),
                      ),
                    );
                  }),
                ),
                const SizedBox(height: 20),
                // ─── Countdown timer ───
                Text(
                  _expired
                      ? 'Kode kadaluarsa'
                      : 'Kode kadaluarsa dalam ',
                  style: TextStyle(
                    fontSize: 13,
                    color: isp.textMuted,
                  ),
                  textAlign: TextAlign.center,
                ),
                if (!_expired)
                  Text(
                    _formattedTime,
                    style: TextStyle(
                      color: isp.warning,
                      fontWeight: FontWeight.w700,
                    ),
                    textAlign: TextAlign.center,
                  ),
                const SizedBox(height: 24),
                // ─── Toggle backup code ───
                Center(
                  child: TextButton(
                    onPressed: () =>
                        setState(() => _usingBackup = !_usingBackup),
                    style: TextButton.styleFrom(
                      foregroundColor: isp.accentLight,
                    ),
                    child: Text(
                      _usingBackup
                          ? l10n.useAuthenticator
                          : l10n.useBackupCode,
                      style: const TextStyle(fontSize: 13),
                    ),
                  ),
                ),
                const SizedBox(height: 16),
                // ─── Verify button ───
                _NeubrutalistAccentButton(
                  label: l10n.verify,
                  loading: _verifying,
                  onTap: _verify,
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
