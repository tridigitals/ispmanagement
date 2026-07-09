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
          // 3 contact method cards — horizontal row (match mockup)
          Row(
            children: [
              _ContactCard(
                  icon: Icons.phone,
                  label: 'Telepon',
                  color: isp.success,
                  onTap: () => _launch('tel:14045')),
              const SizedBox(width: 8),
              _ContactCard(
                  icon: Icons.call,
                  label: 'WhatsApp',
                  color: const Color(0xFF25D366),
                  onTap: () => _launch('https://wa.me/6281234567890')),
              const SizedBox(width: 8),
              _ContactCard(
                  icon: Icons.email_outlined,
                  label: 'Email',
                  color: isp.info,
                  onTap: () => _launch('mailto:support@tridigitals.com')),
            ],
          ),
          const SizedBox(height: 20),
          // Message form
          Text('PESAN',
              style: TextStyle(
                  fontSize: 10,
                  fontWeight: FontWeight.w700,
                  color: isp.textMuted,
                  letterSpacing: 1.2)),
          const SizedBox(height: 6),
          TextField(
            maxLines: 4,
            decoration: InputDecoration(
              hintText: 'Tulis pesan Anda...',
              filled: true,
              fillColor: isp.surface,
              contentPadding: const EdgeInsets.all(14),
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(16),
                borderSide: BorderSide(color: isp.border, width: 1.5),
              ),
              enabledBorder: OutlineInputBorder(
                borderRadius: BorderRadius.circular(16),
                borderSide: BorderSide(color: isp.border, width: 1.5),
              ),
            ),
          ),
          const SizedBox(height: 12),
          // Send button — accent
          SizedBox(
            width: double.infinity,
            child: ElevatedButton(
              onPressed: () {},
              style: ElevatedButton.styleFrom(
                backgroundColor: isp.accent,
                foregroundColor: Colors.white,
                padding: const EdgeInsets.symmetric(vertical: 14),
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(16)),
                elevation: 0,
              ),
              child: const Text('Kirim Pesan',
                  style: TextStyle(fontSize: 14, fontWeight: FontWeight.w700)),
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
  const _ContactCard(
      {required this.icon,
      required this.label,
      required this.color,
      required this.onTap});
  final IconData icon;
  final String label;
  final Color color;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return Expanded(
      child: GestureDetector(
        onTap: onTap,
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 14, horizontal: 8),
          decoration: BoxDecoration(
            color: isp.surface,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: isp.border, width: 1.5),
            boxShadow: [
              BoxShadow(
                  color: isp.border.withOpacity(0.5),
                  offset: Offset(3, 3),
                  blurRadius: 0)
            ],
          ),
          child: Column(
            children: [
              Icon(icon, size: 22, color: color),
              const SizedBox(height: 6),
              Text(label,
                  style: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.w600,
                      color: isp.textPrimary)),
            ],
          ),
        ),
      ),
    );
  }
}
