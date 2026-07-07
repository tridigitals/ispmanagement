import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../l10n/app_localizations.dart';

class ContactScreen extends ConsumerWidget {
  const ContactScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isp = context.isp;
    final l10n = AppLocalizations.of(context);

    return Scaffold(
      backgroundColor: isp.background,
      appBar: AppBar(title: Text(l10n.contactUs), centerTitle: false),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Hero card — contact intro
          Container(
            padding: const EdgeInsets.all(24),
            decoration: BoxDecoration(
              color: isp.surface,
              borderRadius: BorderRadius.circular(14),
              border: Border.all(color: isp.border, width: 1.5),
              boxShadow: [BoxShadow(color: isp.border.withOpacity(0.3), offset: Offset(2, 2))],
            ),
            child: Column(
              children: [
                Container(
                  width: 60, height: 60,
                  decoration: BoxDecoration(
                    color: isp.accent.withOpacity(0.12),
                    shape: BoxShape.circle,
                  ),
                  child: Icon(Icons.support_agent, size: 30, color: isp.accent),
                ),
                const SizedBox(height: 16),
                Text(
                  'Butuh bantuan?',
                  style: TextStyle(fontSize: 18, fontWeight: FontWeight.w800, color: isp.textPrimary),
                ),
                const SizedBox(height: 6),
                Text(
                  'Tim support kami siap membantu Anda',
                  textAlign: TextAlign.center,
                  style: TextStyle(fontSize: 13, color: isp.textSecondary, height: 1.5),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),

          // Contact methods
          _buildSection(isp, [
            _contactTile(
              isp: isp,
              icon: Icons.whatshot,
              iconBg: const Color(0xFF25D366), // WhatsApp green
              title: 'WhatsApp',
              subtitle: 'Chat langsung dengan tim kami',
              onTap: () => _launch('https://wa.me/6281234567890'),
            ),
            _contactTile(
              isp: isp,
              icon: Icons.email_outlined,
              iconBg: isp.info,
              title: 'Email',
              subtitle: 'support@tridigitals.com',
              onTap: () => _launch('mailto:support@tridigitals.com'),
            ),
            _contactTile(
              isp: isp,
              icon: Icons.phone_outlined,
              iconBg: isp.success,
              title: 'Call Center',
              subtitle: '14045 (24 jam)',
              onTap: () => _launch('tel:14045'),
            ),
            _contactTile(
              isp: isp,
              icon: Icons.chat_bubble_outline,
              iconBg: isp.warning,
              title: 'Buat Tiket',
              subtitle: 'Laporkan masalah via tiket',
              onTap: () {
                // Navigate to new ticket
                Navigator.of(context).pushNamed('/tickets/new');
              },
            ),
          ]),

          // Social media
          _sectionHeader(isp, 'Media Sosial'),
          _buildSection(isp, [
            _contactTile(
              isp: isp,
              icon: Icons.camera_alt_outlined,
              iconBg: const Color(0xFFE4405F), // Instagram
              title: 'Instagram',
              subtitle: '@tridigitals.id',
              onTap: () => _launch('https://instagram.com/tridigitals.id'),
            ),
            _contactTile(
              isp: isp,
              icon: Icons.facebook,
              iconBg: const Color(0xFF1877F2), // Facebook
              title: 'Facebook',
              subtitle: 'Tridigitals Indonesia',
              onTap: () => _launch('https://facebook.com/tridigitals'),
            ),
          ]),

          const SizedBox(height: 48),

          // Footer
          Text(
            "Jam operasional: Senin-Jumat 08:00-17:00 WIB\nSabtu 09:00-14:00 WIB",
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 11, color: isp.textMuted, height: 1.5),
          ),
          const SizedBox(height: 32),
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

Widget _sectionHeader(IspThemeColors isp, String label) {
  return Padding(
    padding: const EdgeInsets.fromLTRB(4, 20, 4, 8),
    child: Text(
      label.toUpperCase(),
      style: TextStyle(fontSize: 11, letterSpacing: 1.2, color: isp.textMuted, fontWeight: FontWeight.w700),
    ),
  );
}

Widget _buildSection(IspThemeColors isp, List<Widget> children) {
  return Padding(
    padding: const EdgeInsets.only(bottom: 12),
    child: Container(
      decoration: BoxDecoration(
        color: isp.surface,
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: isp.border, width: 1.5),
        boxShadow: [BoxShadow(color: isp.border.withOpacity(0.3), offset: Offset(2, 2))],
      ),
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

Widget _contactTile({
  required IspThemeColors isp,
  required IconData icon,
  required Color iconBg,
  required String title,
  required String subtitle,
  required VoidCallback onTap,
}) {
  return InkWell(
    onTap: onTap,
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
      child: Row(
        children: [
          Container(
            width: 36, height: 36,
            decoration: BoxDecoration(
              color: iconBg.withOpacity(0.15),
              borderRadius: BorderRadius.circular(10),
            ),
            alignment: Alignment.center,
            child: Icon(icon, size: 19, color: iconBg),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600)),
                const SizedBox(height: 2),
                Text(subtitle, style: TextStyle(fontSize: 11, color: isp.textMuted)),
              ],
            ),
          ),
          Icon(Icons.arrow_forward_ios, size: 14, color: isp.textMuted),
        ],
      ),
    ),
  );
}
