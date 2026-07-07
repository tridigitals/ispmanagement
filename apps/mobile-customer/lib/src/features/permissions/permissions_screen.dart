import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../services/missing_providers.dart';

/// One-time permission request screen shown between onboarding and login.
///
/// Flow: onboarding → permissions → login
class PermissionsScreen extends ConsumerStatefulWidget {
  const PermissionsScreen({super.key});
  @override
  ConsumerState<PermissionsScreen> createState() => _PermissionsScreenState();
}

class _PermissionsScreenState extends ConsumerState<PermissionsScreen> {
  bool _notifRequested = false;
  bool _notifGranted = false;
  bool _notifPermanentlyDenied = false;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback(
        (_) => _requestNotifications());
  }

  Future<void> _requestNotifications() async {
    if (_notifRequested) return;
    _notifRequested = true;
    setState(() {});

    try {
      final settings = await FirebaseMessaging.instance
          .requestPermission(
            alert: true,
            badge: true,
            sound: true,
            provisional: false,
          )
          .timeout(const Duration(seconds: 5), onTimeout: () {
        return NotificationSettings(
          alert: AppleNotificationSetting.enabled,
          announcement: AppleNotificationSetting.enabled,
          authorizationStatus: AuthorizationStatus.notDetermined,
          badge: AppleNotificationSetting.enabled,
          carPlay: AppleNotificationSetting.enabled,
          lockScreen: AppleNotificationSetting.enabled,
          notificationCenter: AppleNotificationSetting.enabled,
          showPreviews: AppleShowPreviewSetting.always,
          timeSensitive: AppleNotificationSetting.enabled,
          criticalAlert: AppleNotificationSetting.disabled,
          sound: AppleNotificationSetting.enabled,
          providesAppNotificationSettings: AppleNotificationSetting.disabled,
        );
      });

      final status = settings.authorizationStatus;
      setState(() {
        _notifGranted = status == AuthorizationStatus.authorized ||
            status == AuthorizationStatus.provisional;
        _notifPermanentlyDenied =
            status == AuthorizationStatus.denied;
      });
    } catch (e) {
      debugPrint('[permissions] notification request failed: $e');
      setState(() {});
    }
  }

  Future<void> _openSettings() async {
    await _requestNotifications();
  }

  Future<void> _continueToLogin() async {
    setState(() => _loading = true);
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setBool('permissions_completed', true);
      if (!mounted) return;
      ref.read(permissionsCompletedProvider.notifier).state = true;
      context.go('/login');
    } catch (e) {
      debugPrint('[permissions] SharedPreferences error: $e');
      if (!mounted) return;
      context.go('/login');
    }
  }

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;

    return Scaffold(
      backgroundColor: isp.background,
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 28),
          child: Column(
            children: [
              Expanded(
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    // ─── Shield icon ───
                    Container(
                      width: 64,
                      height: 64,
                      decoration: NbStyle.card(context), // neubrutalist
                                      // color: isp.surface,
                        borderRadius: BorderRadius.circular(18),
                      ),
                      child: Icon(Icons.shield_outlined,
                          size: 32, color: isp.accentLight),
                    ),
                    const SizedBox(height: 24),
                    Text(
                      'Izinkan Akses',
                      style: const TextStyle(
                        fontSize: 22,
                        fontWeight: FontWeight.w800,
                      ),
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 8),
                    Text(
                      'Aplikasi memerlukan izin berikut untuk pengalaman optimal.',
                      style: TextStyle(
                        color: isp.textMuted,
                        fontSize: 13,
                        height: 1.5,
                      ),
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 32),
                    // ─── Permission cards ───
                    _PermissionCard(
                      icon: Icons.notifications_rounded,
                      title: 'Notifikasi',
                      description: 'Info tagihan & gangguan',
                      status: _notifGranted
                          ? _PermissionStatus.granted
                          : _notifPermanentlyDenied
                              ? _PermissionStatus.permanentlyDenied
                              : _notifRequested
                                  ? _PermissionStatus.denied
                                  : _PermissionStatus.pending,
                      onRetry:
                          _notifPermanentlyDenied ? _openSettings : null,
                      onRequest:
                          !_notifRequested ? _requestNotifications : null,
                    ),
                    const SizedBox(height: 10),
                    _PermissionCard(
                      icon: Icons.location_on_outlined,
                      title: 'Lokasi',
                      description: 'Deteksi jaringan terdekat',
                      status: _PermissionStatus.granted,
                    ),
                    const SizedBox(height: 10),
                    _PermissionCard(
                      icon: Icons.camera_alt_outlined,
                      title: 'Kamera',
                      description: 'Upload foto pendukung',
                      status: _PermissionStatus.granted,
                    ),

                    if (_notifPermanentlyDenied) ...[
                      const SizedBox(height: 12),
                      Container(
                        padding: const EdgeInsets.all(12),
                        decoration: NbStyle.card(context), // neubrutalist
                                        border: Border.all(color: isp.border, width: 1.5),
                                                        boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
                                                        borderRadius:
                              BorderRadius.circular(IspRadii.md),
                          border: Border.all(
                            color: isp.danger.withOpacity(0.2),
                          ),
                        ),
                        child: Row(
                          children: [
                            Icon(Icons.info_outline,
                                color: isp.danger, size: 20),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                'Notifikasi ditolak permanen. Buka Settings → Apps → ISP Customer → Notifications untuk mengaktifkannya.',
                                style: TextStyle(
                                  color: isp.danger,
                                  fontSize: 12,
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ],
                ),
              ),
              // ─── Bottom button ───
              Padding(
                padding: const EdgeInsets.only(bottom: 32),
                child: _NeubrutalistAccentButton(
                  label: _loading
                      ? 'Memuat...'
                      : _notifGranted
                          ? 'Lanjutkan'
                          : 'Izinkan Semua',
                  loading: _loading,
                  onTap: _loading ? null : _continueToLogin,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

enum _PermissionStatus { pending, granted, denied, permanentlyDenied }

class _PermissionCard extends StatelessWidget {
  const _PermissionCard({
    required this.icon,
    required this.title,
    required this.description,
    required this.status,
    this.onRequest,
    this.onRetry,
  });

  final IconData icon;
  final String title;
  final String description;
  final _PermissionStatus status;
  final VoidCallback? onRequest;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;

    Color statusColor;
    String statusText;
    IconData statusIcon;

    switch (status) {
      case _PermissionStatus.granted:
        statusColor = isp.success;
        statusText = 'Diaktifkan';
        statusIcon = Icons.check_circle_rounded;
      case _PermissionStatus.denied:
        statusColor = isp.warning;
        statusText = 'Ditolak';
        statusIcon = Icons.cancel_rounded;
      case _PermissionStatus.permanentlyDenied:
        statusColor = isp.danger;
        statusText = 'Ditolak Permanen';
        statusIcon = Icons.block_rounded;
      case _PermissionStatus.pending:
        statusColor = isp.textMuted;
        statusText = 'Menunggu...';
        statusIcon = Icons.hourglass_empty_rounded;
    }

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: NbStyle.card(context), // neubrutalist
                      // color: isp.surface,
        border: Border.all(color: isp.border, width: 1.5),
                        boxShadow: [BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0)],
                        borderRadius:
        boxShadow: [
          BoxShadow(
            offset: const Offset(3, 3),
            decoration: NbStyle.card(context), // neubrutalist
                            // color: isp.surface,
          ),
        ],
      ),
      child: Row(
        children: [
          Container(
            width: 44,
            height: 44,
            decoration: NbStyle.card(context), // neubrutalist
                            // color: isp.surface,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Icon(icon, color: statusColor, size: 22),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: const TextStyle(
                    fontWeight: FontWeight.w600,
                    fontSize: 14,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  description,
                  style: TextStyle(
                    color: isp.textMuted,
                    fontSize: 11,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(statusIcon, color: statusColor, size: 22),
              const SizedBox(height: 2),
              Text(
                statusText,
                style: TextStyle(
                  color: statusColor,
                  fontSize: 10,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ],
          ),
        ],
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
        decoration: NbStyle.card(context), // neubrutalist
                        // color: isp.surface,
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

