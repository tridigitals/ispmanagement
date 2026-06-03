import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';

class ContactScreen extends StatelessWidget {
  const ContactScreen({super.key});

  // Replace these with the actual ISP contact details.
  static const _callCenter = '08001234567';
  static const _whatsapp = '6281234567890';
  static const _email = 'cs@tridigitals.com';
  static const _address = 'Jl. Jendral Sudirman No.1, Jakarta';

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(title: Text(l10n.contactUs)),
      body: ListView(
        padding: const EdgeInsets.all(IspSpacing.lg),
        children: [
          _ContactCard(
            icon: Icons.phone,
            color: IspColors.success,
            title: 'Telepon',
            subtitle: '$_callCenter (24/7)',
            onTap: () => _launch('tel:$_callCenter'),
          ),
          const SizedBox(height: 12),
          _ContactCard(
            icon: Icons.chat_bubble,
            color: IspColors.success,
            title: 'WhatsApp',
            subtitle: '+$_whatsapp',
            onTap: () => _launch('https://wa.me/$_whatsapp'),
          ),
          const SizedBox(height: 12),
          _ContactCard(
            icon: Icons.email_outlined,
            color: IspColors.info,
            title: 'Email',
            subtitle: _email,
            onTap: () => _launch('mailto:$_email'),
          ),
          const SizedBox(height: 12),
          _ContactCard(
            icon: Icons.location_on_outlined,
            color: IspColors.primary,
            title: l10n.officeAddress,
            subtitle: _address,
            onTap: () =>
                _launch('https://maps.google.com/?q=${Uri.encodeComponent(_address)}'),
          ),
          const SizedBox(height: 24),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                children: [
                  const Icon(Icons.access_time, color: IspColors.textTertiary),
                  const SizedBox(height: 8),
                  Text(
                    l10n.serviceHours,
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(height: 4),
                  const Text(
                    'Senin - Minggu\n24 jam (gangguan)\n08:00 - 21:00 (admin)',
                    textAlign: TextAlign.center,
                    style: TextStyle(color: IspColors.textTertiary),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _launch(String url) async {
    final uri = Uri.parse(url);
    if (await canLaunchUrl(uri)) {
      await launchUrl(uri, mode: LaunchMode.externalApplication);
    }
  }
}

class _ContactCard extends StatelessWidget {
  const _ContactCard({
    required this.icon,
    required this.color,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });
  final IconData icon;
  final Color color;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(IspRadii.lg),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: color.withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(IspRadii.md),
                ),
                child: Icon(icon, color: color),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      style: const TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      subtitle,
                      style: const TextStyle(
                        fontSize: 13,
                        color: IspColors.textTertiary,
                      ),
                    ),
                  ],
                ),
              ),
              const Icon(Icons.chevron_right, color: IspColors.textTertiary),
            ],
          ),
        ),
      ),
    );
  }
}
