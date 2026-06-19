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
    return Scaffold(
      appBar: AppBar(title: Text(l10n.settings)),
      body: ListView(
        children: [
          const SizedBox(height: IspSpacing.sm),
          _SectionHeader(label: l10n.account),
          IspCard(
            margin: const EdgeInsets.symmetric(
                horizontal: IspSpacing.md, vertical: IspSpacing.xs),
            child: Column(
              children: [
                _SwitchTile(
                  icon: Icons.fingerprint,
                  title: l10n.biometric,
                  subtitle: l10n.biometricSub,
                  value:
                      ref.watch(biometricEnabledProvider).valueOrNull ?? false,
                  onChanged: (v) => _toggleBiometric(context, ref, v),
                ),
                Divider(height: 1, color: isp.borderSubtle),
                _SwitchTile(
                  icon: Icons.security,
                  title: l10n.twoFactorAuth,
                  subtitle: user?.twoFactorEnabled == true
                      ? (user?.enforce2fa == true
                          ? l10n.twoFaRequired
                          : l10n.twoFaOn)
                      : l10n.twoFaOff,
                  value: user?.twoFactorEnabled == true,
                  onChanged: user?.twoFactorEnabled == true &&
                          user?.enforce2fa == true
                      ? null // Prevent disabling when enforced
                      : (v) => _toggle2fa(context, ref, v),
                ),
                Divider(height: 1, color: isp.borderSubtle),
                ListTile(
                  leading: const Icon(Icons.lock_outline),
                  title: Text(l10n.changePassword),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => GoRouter.of(context).push('/change-password'),
                ),
                Divider(height: 1, color: isp.borderSubtle),
                ListTile(
                  leading: const Icon(Icons.edit_outlined),
                  title: Text(l10n.editProfile),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => GoRouter.of(context).push('/edit-profile'),
                ),
              ],
            ),
          ),
          const SizedBox(height: IspSpacing.lg),
          _SectionHeader(label: 'Bahasa / Language'),
          IspCard(
            margin: const EdgeInsets.symmetric(
                horizontal: IspSpacing.md, vertical: IspSpacing.xs),
            child: Column(
              children: [
                RadioListTile<Locale>(
                  secondary: const Icon(Icons.language),
                  title: const Text('Bahasa Indonesia'),
                  value: const Locale('id'),
                  groupValue: Localizations.localeOf(context),
                  onChanged: (locale) {
                    if (locale != null) {
                      ref.read(localeProvider.notifier).setLocale(locale);
                    }
                  },
                ),
                Divider(height: 1, color: isp.borderSubtle),
                RadioListTile<Locale>(
                  secondary: const Icon(Icons.language),
                  title: const Text('English'),
                  value: const Locale('en'),
                  groupValue: Localizations.localeOf(context),
                  onChanged: (locale) {
                    if (locale != null) {
                      ref.read(localeProvider.notifier).setLocale(locale);
                    }
                  },
                ),
              ],
            ),
          ),
          const SizedBox(height: IspSpacing.lg),
          _SectionHeader(label: l10n.about),
          IspCard(
            margin: const EdgeInsets.symmetric(
                horizontal: IspSpacing.md, vertical: IspSpacing.xs),
            child: Column(
              children: [
                ListTile(
                  leading: const Icon(Icons.privacy_tip_outlined),
                  title: Text(l10n.privacyPolicy),
                  trailing: const Icon(Icons.open_in_new),
                  onTap: () => _openUrl('https://tridigitals.com/privacy'),
                ),
                Divider(height: 1, color: isp.borderSubtle),
                ListTile(
                  leading: const Icon(Icons.description_outlined),
                  title: Text(l10n.termsOfService),
                  trailing: const Icon(Icons.open_in_new),
                  onTap: () => _openUrl('https://tridigitals.com/terms'),
                ),
                Divider(height: 1, color: isp.borderSubtle),
                ListTile(
                  leading: Icon(Icons.info_outline),
                  title: Text('Versi Aplikasi'),
                  trailing: Text(
                    '0.1.0+1',
                    style: TextStyle(color: isp.textMuted),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: IspSpacing.xxl),
        ],
      ),
    );
  }

  Future<void> _toggleBiometric(
    BuildContext context,
    WidgetRef ref,
    bool enable,
  ) async {
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
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(e.toString())),
          );
        }
      }
    } else {
      await ref.read(biometricEnabledProvider.notifier).set(false);
    }
  }

  Future<void> _toggle2fa(
    BuildContext context,
    WidgetRef ref,
    bool enable,
  ) async {
    if (enable) {
      GoRouter.of(context).push('/security/2fa/enroll');
    } else {
      // Confirm before disabling
      final confirm = await showDialog<bool>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: Text(AppLocalizations.of(ctx).disable2faConfirmTitle),
          content: Text(
            AppLocalizations.of(ctx).disable2faConfirmBody,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: Text(AppLocalizations.of(ctx).cancel),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              style: FilledButton.styleFrom(
                backgroundColor: ctx.isp.danger,
              ),
              child: Text(
                AppLocalizations.of(ctx).disable,
              ),
            ),
          ],
        ),
      );
      if (confirm == true && context.mounted) {
        await _processDisable2fa(context, ref);
      }
    }
  }

  Future<void> _processDisable2fa(
    BuildContext context,
    WidgetRef ref,
  ) async {
    final l10n = AppLocalizations.of(context);

    // First attempt - request OTP
    final result = await ref.read(authControllerProvider.notifier).disable2fa();

    if (result is Failure) {
      final error = (result as Failure).exception;

      // Check if this is the "requires_verification" signal
      if (error.message == 'requires_verification' && context.mounted) {
        // Show OTP dialog
        final code = await _showOtpDialog(context);
        if (code != null && code.isNotEmpty && context.mounted) {
          // Second attempt with OTP code
          final result2 = await ref
              .read(authControllerProvider.notifier)
              .disable2fa(code: code);

          if (result2 is Success) {
            if (context.mounted) {
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(content: Text(l10n.twoFaDisabledSuccess)),
              );
            }
          } else if (result2 is Failure && context.mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text((result2 as Failure).exception.message),
                backgroundColor: context.isp.danger,
              ),
            );
          }
        }
      } else if (context.mounted) {
        // Show other errors
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(error.message),
            backgroundColor: context.isp.danger,
          ),
        );
      }
    } else if (result is Success && context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.twoFaDisabledSuccess)),
      );
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
            Text(
              l10n.otpSentToEmail,
              style: TextStyle(color: ctx.isp.textMuted, fontSize: 13),
            ),
            const SizedBox(height: 16),
            TextField(
              controller: controller,
              autofocus: true,
              keyboardType: TextInputType.number,
              maxLength: 6,
              decoration: InputDecoration(
                labelText: l10n.verificationCode,
                hintText: '123456',
                border: const OutlineInputBorder(),
              ),
              onSubmitted: (value) => Navigator.pop(ctx, value),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, controller.text),
            child: Text(l10n.verify),
          ),
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

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({required this.label});
  final String label;
  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return Padding(
      padding: const EdgeInsets.fromLTRB(20, IspSpacing.lg, 20, IspSpacing.sm),
      child: Text(
        label.toUpperCase(),
        style: TextStyle(
          fontSize: 11,
          letterSpacing: 1.2,
          color: isp.textMuted,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _SwitchTile extends StatelessWidget {
  const _SwitchTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.value,
    required this.onChanged,
  });
  final IconData icon;
  final String title;
  final String subtitle;
  final bool value;
  final ValueChanged<bool>? onChanged;
  @override
  Widget build(BuildContext context) {


    final isp = context.isp;    return SwitchListTile(
      secondary: Icon(icon),
      title: Text(title),
      subtitle: Text(subtitle),
      value: value,
      onChanged: onChanged,
    );
  }
}
