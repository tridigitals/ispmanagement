import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:local_auth/local_auth.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/auth_providers.dart';
import '../../services/feature_providers.dart';
import '../../services/missing_providers.dart';
import '../../services/settings_providers.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context)!;
    final user = ref.watch(currentUserProvider);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.settings)),
      body: ListView(
        children: [
          const SizedBox(height: IspSpacing.sm),
          _SectionHeader(label: l10n.account),
          IspCard(
            margin: const EdgeInsets.symmetric(horizontal: IspSpacing.md, vertical: IspSpacing.xs),
            child: Column(
              children: [
                _SwitchTile(
                  icon: Icons.fingerprint,
                  title: l10n.biometric,
                  subtitle: l10n.biometricSub,
                  value: ref.watch(biometricEnabledProvider).valueOrNull ?? false,
                  onChanged: (v) => _toggleBiometric(context, ref, v),
                ),
                const Divider(height: 1, color: IspColors.borderSubtle),
                _SwitchTile(
                  icon: Icons.security,
                  title: l10n.twoFactorAuth,
                  subtitle: user?.twoFactorEnabled == true
                      ? l10n.twoFaOn
                      : l10n.twoFaOff,
                  value: user?.twoFactorEnabled == true,
                  onChanged: (v) => _toggle2fa(context, ref, v),
                ),
                const Divider(height: 1, color: IspColors.borderSubtle),
                ListTile(
                  leading: const Icon(Icons.lock_outline),
                  title: Text(l10n.changePassword),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => context.push('/change-password'),
                ),
                const Divider(height: 1, color: IspColors.borderSubtle),
                ListTile(
                  leading: const Icon(Icons.edit_outlined),
                  title: Text(l10n.editProfile),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => context.push('/edit-profile'),
                ),
              ],
            ),
          ),
          const SizedBox(height: IspSpacing.lg),
          _SectionHeader(label: l10n.notifications),
          IspCard(
            margin: const EdgeInsets.symmetric(horizontal: IspSpacing.md, vertical: IspSpacing.xs),
            child: Column(
              children: [
                _SwitchTile(
                  icon: Icons.receipt_long,
                  title: l10n.notifInvoice,
                  subtitle: l10n.notifInvoiceSub,
                  value: ref.watch(notifInvoiceEnabledProvider),
                  onChanged: (v) => ref
                      .read(notifInvoiceEnabledProvider.notifier)
                      .set(v),
                ),
                const Divider(height: 1, color: IspColors.borderSubtle),
                _SwitchTile(
                  icon: Icons.warning_amber,
                  title: l10n.notifOutage,
                  subtitle: l10n.notifOutageSub,
                  value: ref.watch(notifOutageEnabledProvider),
                  onChanged: (v) => ref
                      .read(notifOutageEnabledProvider.notifier)
                      .set(v),
                ),
                const Divider(height: 1, color: IspColors.borderSubtle),
                _SwitchTile(
                  icon: Icons.local_offer,
                  title: l10n.notifPromo,
                  subtitle: l10n.notifPromoSub,
                  value: ref.watch(notifPromoEnabledProvider),
                  onChanged: (v) => ref
                      .read(notifPromoEnabledProvider.notifier)
                      .set(v),
                ),
              ],
            ),
          ),
          const SizedBox(height: IspSpacing.lg),
          _SectionHeader(label: 'Bahasa / Language'),
          IspCard(
            margin: const EdgeInsets.symmetric(horizontal: IspSpacing.md, vertical: IspSpacing.xs),
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
                const Divider(height: 1, color: IspColors.borderSubtle),
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
            margin: const EdgeInsets.symmetric(horizontal: IspSpacing.md, vertical: IspSpacing.xs),
            child: Column(
              children: [
                ListTile(
                  leading: const Icon(Icons.privacy_tip_outlined),
                  title: Text(l10n.privacyPolicy),
                  trailing: const Icon(Icons.open_in_new),
                  onTap: () => _openUrl('https://tridigitals.com/privacy'),
                ),
                const Divider(height: 1, color: IspColors.borderSubtle),
                ListTile(
                  leading: const Icon(Icons.description_outlined),
                  title: Text(l10n.termsOfService),
                  trailing: const Icon(Icons.open_in_new),
                  onTap: () => _openUrl('https://tridigitals.com/terms'),
                ),
                const Divider(height: 1, color: IspColors.borderSubtle),
                const ListTile(
                  leading: Icon(Icons.info_outline),
                  title: Text('Versi Aplikasi'),
                  trailing: Text(
                    '0.1.0+1',
                    style: TextStyle(color: IspColors.textTertiary),
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
    if (enable) {
      final auth = LocalAuthentication();
      try {
        final canCheck = await auth.canCheckBiometrics;
        if (!canCheck) {
          if (context.mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text(
                  AppLocalizations.of(context)!.biometricNotAvailable,
                ),
              ),
            );
          }
          return;
        }
        final ok = await auth.authenticate(
          localizedReason:
              AppLocalizations.of(context)!.biometricEnableReason,
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
      context.push('/security/2fa/enroll');
    } else {
      // Confirm before disabling
      final confirm = await showDialog<bool>(
        context: context,
        builder: (_) => AlertDialog(
          title: Text(AppLocalizations.of(context)!.disable2faConfirmTitle),
          content: Text(
            AppLocalizations.of(context)!.disable2faConfirmBody,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: Text(AppLocalizations.of(context)!.cancel),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              style: FilledButton.styleFrom(
                backgroundColor: IspColors.danger,
              ),
              child: Text(
                AppLocalizations.of(context)!.disable,
              ),
            ),
          ],
        ),
      );
      if (confirm == true) {
        await ref
            .read(authControllerProvider.notifier)
            .disable2fa();
      }
    }
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
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, IspSpacing.lg, 20, IspSpacing.sm),
      child: Text(
        label.toUpperCase(),
        style: const TextStyle(
          fontSize: 11,
          letterSpacing: 1.2,
          color: IspColors.textTertiary,
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
  final ValueChanged<bool> onChanged;
  @override
  Widget build(BuildContext context) {
    return SwitchListTile(
      secondary: Icon(icon),
      title: Text(title),
      subtitle: Text(subtitle),
      value: value,
      onChanged: onChanged,
    );
  }
}
