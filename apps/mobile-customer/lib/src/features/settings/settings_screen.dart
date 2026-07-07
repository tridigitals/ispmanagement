import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:local_auth/local_auth.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:ui_kit/ui_kit.dart';
import 'package:api_client/api_client.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../services/missing_providers.dart';
import '../../services/settings_providers.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final user = ref.watch(currentUserProvider);
    final bio = ref.watch(biometricEnabledProvider).valueOrNull ?? false;

    return Scaffold(
      backgroundColor: isp.background,
      appBar: AppBar(title: Text(l10n.settings), centerTitle: false),
      body: ListView(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        children: [
          const SizedBox(height: 8),

          // Account section
          _sectionHeader(isp, l10n.account),
          _buildSection(context, isp, [
            _switchTile(isp, Icons.fingerprint, l10n.biometric, l10n.biometricSub, bio,
                (v) => _toggleBiometric(context, ref, v)),
            _switchTile(isp, Icons.security, l10n.twoFactorAuth,
                user?.twoFactorEnabled == true
                    ? (user?.enforce2fa == true ? l10n.twoFaRequired : l10n.twoFaOn)
                    : l10n.twoFaOff,
                user?.twoFactorEnabled == true,
                user?.twoFactorEnabled == true && user?.enforce2fa == true
                    ? null
                    : (v) => _toggle2fa(context, ref, v)),
            _navTile(context, isp, Icons.lock_outline, l10n.changePassword,
                () => GoRouter.of(context).push('/change-password')),
            _navTile(context, isp, Icons.edit_outlined, l10n.editProfile,
                () => GoRouter.of(context).push('/edit-profile')),
          ]),

          // Language section
          _sectionHeader(isp, 'Bahasa / Language'),
          _buildSection(context, isp, [
            _radioTile(context, isp, Icons.translate, 'Bahasa Indonesia', const Locale('id'),
                Localizations.localeOf(context),
                (loc) => ref.read(localeProvider.notifier).setLocale(loc!)),
            _radioTile(context, isp, Icons.translate, 'English', const Locale('en'),
                Localizations.localeOf(context),
                (loc) => ref.read(localeProvider.notifier).setLocale(loc!)),
          ]),

          // About section
          _sectionHeader(isp, l10n.about),
          _buildSection(context, isp, [
            _navTile(context, isp, Icons.privacy_tip_outlined, l10n.privacyPolicy,
                () => _openUrl('https://tridigitals.com/privacy')),
            _navTile(context, isp, Icons.description_outlined, l10n.termsOfService,
                () => _openUrl('https://tridigitals.com/terms')),
            _infoTile(context, isp, Icons.info_outline, 'Versi Aplikasi', '0.1.0+1'),
          ]),

          const SizedBox(height: 48),
        ],
      ),
    );
  }

  Future<void> _toggleBiometric(BuildContext context, WidgetRef ref, bool enable) async {
    final l10n = AppLocalizations.of(context);
    if (enable) {
      final auth = LocalAuthentication();
      try {
        final canCheck = await auth.canCheckBiometrics;
        if (!canCheck) {
          if (context.mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(content: Text(l10n.biometricNotAvailable)),
            );
          }
          return;
        }
        final ok = await auth.authenticate(
          localizedReason: l10n.biometricEnableReason,
          options: const AuthenticationOptions(stickyAuth: true),
        );
        if (ok) {
          await ref.read(biometricEnabledProvider.notifier).set(true);
        }
      } catch (e) {
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(e.toString())));
        }
      }
    } else {
      await ref.read(biometricEnabledProvider.notifier).set(false);
    }
  }

  Future<void> _toggle2fa(BuildContext context, WidgetRef ref, bool enable) async {
    if (enable) {
      GoRouter.of(context).push('/security/2fa/enroll');
    } else {
      final confirm = await showDialog<bool>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: Text(AppLocalizations.of(ctx).disable2faConfirmTitle),
          content: Text(AppLocalizations.of(ctx).disable2faConfirmBody),
          actions: [
            TextButton(onPressed: () => Navigator.pop(ctx, false), child: Text(AppLocalizations.of(ctx).cancel)),
            FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              style: FilledButton.styleFrom(backgroundColor: ctx.isp.danger),
              child: Text(AppLocalizations.of(ctx).disable),
            ),
          ],
        ),
      );
      if (confirm == true && context.mounted) {
        await _processDisable2fa(context, ref);
      }
    }
  }

  Future<void> _processDisable2fa(BuildContext context, WidgetRef ref) async {
    final l10n = AppLocalizations.of(context);
    final result = await ref.read(authControllerProvider.notifier).disable2fa();
    if (result is Failure) {
      final error = (result as Failure).exception;
      if (error.message == 'requires_verification' && context.mounted) {
        final code = await _showOtpDialog(context);
        if (code != null && code.isNotEmpty && context.mounted) {
          final result2 = await ref.read(authControllerProvider.notifier).disable2fa(code: code);
          if (result2 is Success && context.mounted) {
            ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(l10n.twoFaDisabledSuccess)));
          } else if (result2 is Failure && context.mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(content: Text((result2 as Failure).exception.message), backgroundColor: context.isp.danger),
            );
          }
        }
      } else if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(error.message), backgroundColor: context.isp.danger),
        );
      }
    } else if (result is Success && context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(l10n.twoFaDisabledSuccess)));
    }
  }

  Future<String?> _showOtpDialog(BuildContext context) async {
    final l10n = AppLocalizations.of(context);
    final controller = TextEditingController();
    return showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l10n.enterVerificationCode),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(l10n.otpSentToEmail, style: TextStyle(color: ctx.isp.textMuted, fontSize: 13)),
            const SizedBox(height: 16),
            TextField(
              controller: controller, autofocus: true, keyboardType: TextInputType.number, maxLength: 6,
              decoration: InputDecoration(
                labelText: l10n.verificationCode, hintText: '123456', border: const OutlineInputBorder(),
              ),
              onSubmitted: (value) => Navigator.pop(ctx, value),
            ),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: Text(l10n.cancel)),
          FilledButton(onPressed: () => Navigator.pop(ctx, controller.text), child: Text(l10n.verify)),
        ],
      ),
    );
  }

  Future<void> _openUrl(String url) async {
    final uri = Uri.parse(url);
    if (await canLaunchUrl(uri)) {
      await launchUrl(uri, mode: LaunchMode.externalApplication);
    }
  }
}

// ─── Helpers ──────────────────────────────────────────────────────

Widget _sectionHeader(IspThemeColors isp, String label) {
  return Padding(
    padding: const EdgeInsets.fromLTRB(4, 20, 4, 8),
    child: Text(
      label.toUpperCase(),
      style: TextStyle(fontSize: 11, letterSpacing: 1.2, color: isp.textMuted, fontWeight: FontWeight.w700),
    ),
  );
}

Widget _buildSection(BuildContext context, IspThemeColors isp, List<Widget> children) {
  return Padding(
    padding: const EdgeInsets.only(bottom: 12),
    child: Container(
      decoration: NbStyle.card(context, radius: BorderRadius.circular(14)),
      clipBehavior: Clip.antiAlias,
      child: Column(
        children: [
          for (var i = 0; i < children.length; i++) ...[
            children[i],
            if (i < children.length - 1) Divider(height: 1, indent: 56, color: isp.borderSubtle),
          ],
        ],
      ),
    ),
  );
}

Widget _switchTile(IspThemeColors isp, IconData icon, String title, String sub, bool value, ValueChanged<bool>? onChanged) {
  return Padding(
    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 2),
    child: SwitchListTile(
      secondary: Icon(icon, size: 20, color: isp.accent),
      title: Text(title, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600)),
      subtitle: Text(sub, style: TextStyle(fontSize: 11, color: isp.textMuted)),
      value: value,
      onChanged: onChanged,
      dense: true,
    ),
  );
}

Widget _navTile(BuildContext context, IspThemeColors isp, IconData icon, String title, VoidCallback onTap) {
  return InkWell(
    onTap: onTap,
    child: Padding(
    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
    child: Row(
      children: [
        NbStyle.iconContainer(context, icon, color: isp.accent, size: 17),
        const SizedBox(width: 12),
          Expanded(child: Text(title, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600))),
          Icon(Icons.chevron_right, size: 18, color: isp.textMuted),
        ],
      ),
    ),
  );
}

Widget _radioTile(BuildContext context, IspThemeColors isp, IconData icon, String title, Locale value, Locale group, ValueChanged<Locale?> onChanged) {
  return InkWell(
    onTap: () => onChanged(value),
    child: Padding(
    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
    child: Row(
      children: [
        NbStyle.iconContainer(context, icon, color: isp.info, size: 17),
        const SizedBox(width: 12),
          Expanded(child: Text(title, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600))),
          Icon(
            value == group ? Icons.radio_button_checked : Icons.radio_button_off,
            size: 20, color: value == group ? isp.accent : isp.textMuted,
          ),
        ],
      ),
    ),
  );
}

Widget _infoTile(BuildContext context, IspThemeColors isp, IconData icon, String title, String value) {
  return Padding(
    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
    child: Row(
      children: [
        NbStyle.iconContainer(context, icon, color: isp.textMuted, size: 17),
        const SizedBox(width: 12),
        Expanded(child: Text(title, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600))),
        Text(value, style: TextStyle(color: isp.textMuted, fontSize: 13)),
      ],
    ),
  );
}
