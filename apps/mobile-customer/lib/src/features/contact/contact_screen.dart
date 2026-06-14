import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';
import '../../services/app_config.dart';

/// Fetch contact info from portal API.
final contactInfoProvider = FutureProvider<Map<String, String>>((ref) async {
  final dio = ref.read(dioProvider);
  try {
    final res =
        await dio.get<Map<String, dynamic>>('/api/customers/portal/contact');
    final data = res.data ?? {};
    return data.map((k, v) => MapEntry(k, v?.toString() ?? ''));
  } catch (_) {
    return {};
  }
});

class ContactScreen extends ConsumerWidget {
  const ContactScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);
    final contactAsync = ref.watch(contactInfoProvider);

    return Scaffold(
      appBar: AppBar(title: Text(l10n.contactUs)),
      body: contactAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (_, __) => _buildFallback(context, l10n, {}),
        data: (contact) => _buildFallback(context, l10n, contact),
      ),
    );
  }

  Widget _buildFallback(
    BuildContext context,
    AppLocalizations l10n,
    Map<String, String> contact,
  ) {
    final isp = context.isp;
    final phone = contact['company_phone'] ?? '';
    final whatsapp = contact['company_whatsapp'] ?? '';
    final email = contact['company_email'] ?? '';
    final address = contact['company_address'] ?? '';
    final website = contact['company_website'] ?? '';
    final orgName = contact['organization_name'] ?? '';

    return ListView(
      padding: const EdgeInsets.all(IspSpacing.lg),
      children: [
        if (orgName.isNotEmpty) ...[
          Text(
            orgName,
            style: const TextStyle(
              fontSize: 18,
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 16),
        ],
        if (phone.isNotEmpty)
          _ContactCard(
            icon: Icons.phone,
            color: isp.success,
            title: 'Telepon',
            subtitle: phone,
            onTap: () => _launch('tel:$phone'),
          ),
        if (phone.isNotEmpty) const SizedBox(height: 12),
        if (whatsapp.isNotEmpty)
          _ContactCard(
            icon: Icons.chat_bubble,
            color: isp.success,
            title: 'WhatsApp',
            subtitle: whatsapp,
            onTap: () {
              final waNumber = whatsapp.replaceAll(RegExp(r'[^0-9]'), '');
              _launch('https://wa.me/$waNumber');
            },
          ),
        if (whatsapp.isNotEmpty) const SizedBox(height: 12),
        if (email.isNotEmpty)
          _ContactCard(
            icon: Icons.email_outlined,
            color: isp.info,
            title: 'Email',
            subtitle: email,
            onTap: () => _launch('mailto:$email'),
          ),
        if (email.isNotEmpty) const SizedBox(height: 12),
        if (address.isNotEmpty)
          _ContactCard(
            icon: Icons.location_on_outlined,
            color: isp.accent,
            title: l10n.officeAddress,
            subtitle: address,
            onTap: () => _launch(
                'https://maps.google.com/?q=${Uri.encodeComponent(address)}'),
          ),
        if (website.isNotEmpty) ...[
          const SizedBox(height: 12),
          _ContactCard(
            icon: Icons.language,
            color: isp.info,
            title: 'Website',
            subtitle: website,
            onTap: () => _launch(website),
          ),
        ],
        if (phone.isEmpty && whatsapp.isEmpty && email.isEmpty) ...[
          const SizedBox(height: 48),
          Center(
            child: Column(
              children: [
                Icon(Icons.contact_support_outlined,
                    size: 48, color: isp.textMuted),
                SizedBox(height: 12),
                Text(
                  'Informasi kontak belum tersedia',
                  style: TextStyle(color: isp.textMuted),
                ),
              ],
            ),
          ),
        ],
        const SizedBox(height: 24),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                Icon(Icons.access_time, color: isp.textMuted),
                const SizedBox(height: 8),
                Text(
                  l10n.serviceHours,
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 4),
                Text(
                  'Senin - Minggu\n24 jam (gangguan)\n08:00 - 21:00 (admin)',
                  textAlign: TextAlign.center,
                  style: TextStyle(color: isp.textMuted),
                ),
              ],
            ),
          ),
        ),
      ],
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


    final isp = context.isp;    return Card(
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
                  color: color.withOpacity(0.12),
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
                      style: TextStyle(
                        fontSize: 13,
                        color: isp.textMuted,
                      ),
                    ),
                  ],
                ),
              ),
              Icon(Icons.chevron_right, color: isp.textMuted),
            ],
          ),
        ),
      ),
    );
  }
}
