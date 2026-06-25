import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:ui_kit/ui_kit.dart';

import '../../services/missing_providers.dart';
import '../../services/fcm_service.dart';

/// One-time permission request screen shown between onboarding and login.
///
/// Explains why each permission is needed, then requests them upfront so the
/// user knows what they're agreeing to — not buried deep inside the app.
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
  bool _notifDenied = false;
  bool _notifPermanentlyDenied = false;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _requestNotifications());
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
            status == AuthorizationStatus.denied && !_notifGranted;
        _notifDenied = status == AuthorizationStatus.denied && !_notifGranted;
      });
    } catch (e) {
      debugPrint('[permissions] notification request failed: $e');
      setState(() => _notifDenied = true);
    }
  }

  Future<void> _openSettings() async {
    // Re-request notification permission.
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
    final l10n = _L10n._(context);

    return Scaffold(
      body: SafeArea(
        child: Column(
          children: [
            // Top — illustration
            Expanded(
              flex: 2,
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 120,
                      height: 120,
                      decoration: BoxDecoration(
                        color: isp.accent.withOpacity(0.12),
                        shape: BoxShape.circle,
                      ),
                      child: Icon(
                        Icons.notifications_active_rounded,
                        size: 56,
                        color: isp.accent,
                      ),
                    ),
                    const SizedBox(height: 24),
                    Text(
                      'Aktifkan Notifikasi',
                      style: const TextStyle(
                        fontSize: 22,
                        fontWeight: FontWeight.w700,
                      ),
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 8),
                    Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 40),
                      child: Text(
                        'Dapatkan push notification untuk tiket baru, update work order, dan info penting saat di lapangan.',
                        style: TextStyle(
                          color: isp.textMuted,
                          fontSize: 14,
                        ),
                        textAlign: TextAlign.center,
                      ),
                    ),
                  ],
                ),
              ),
            ),

            // Permission item card
            Expanded(
              flex: 3,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: IspSpacing.lg),
                child: Column(
                  children: [
                    _PermissionCard(
                      icon: Icons.notifications_rounded,
                      title: 'Notifikasi Push',
                      description:
                          'Terima pengingat tagihan, update tiket, dan informasi gangguan jaringan.',
                      status: _notifGranted
                          ? _PermissionStatus.granted
                          : _notifPermanentlyDenied
                              ? _PermissionStatus.permanentlyDenied
                              : _notifRequested
                                  ? _PermissionStatus.denied
                                  : _PermissionStatus.pending,
                      onRetry: _notifPermanentlyDenied ? _openSettings : null,
                      onRequest:
                          !_notifRequested ? _requestNotifications : null,
                    ),

                    if (_notifPermanentlyDenied) ...[
                      const SizedBox(height: 8),
                      Container(
                        padding: const EdgeInsets.all(12),
                        decoration: BoxDecoration(
                          color: isp.danger.withOpacity(0.08),
                          borderRadius: BorderRadius.circular(IspRadii.md),
                          border: Border.all(
                            color: isp.danger.withOpacity(0.2),
                          ),
                        ),
                        child: Row(
                          children: [
                            Icon(
                              Icons.info_outline,
                              color: isp.danger,
                              size: 20,
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                'Notifikasi ditolak permanen. Buka Settings → Apps → ISP Teknisi → Notifications untuk mengaktifkannya.',
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

                    const Spacer(),

                    // Continue button — always enabled so user can proceed
                    IspPrimaryButton(
                      label: _loading
                          ? 'Memuat...'
                          : _notifGranted
                              ? 'Lanjutkan'
                              : 'Lewati Saja',
                      icon: _loading
                          ? null
                          : _notifGranted
                              ? Icons.check
                              : Icons.arrow_forward,
                      onPressed: _loading ? null : _continueToLogin,
                      loading: _loading,
                    ),
                    const SizedBox(height: IspSpacing.lg),
                  ],
                ),
              ),
            ),
          ],
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
        statusColor = const Color(0xFF22C55E);
        statusText = 'Diaktifkan';
        statusIcon = Icons.check_circle_rounded;
      case _PermissionStatus.denied:
        statusColor = const Color(0xFFF59E0B);
        statusText = 'Ditolak';
        statusIcon = Icons.cancel_rounded;
      case _PermissionStatus.permanentlyDenied:
        statusColor = const Color(0xFFEF4444);
        statusText = 'Ditolak Permanen';
        statusIcon = Icons.block_rounded;
      case _PermissionStatus.pending:
        statusColor = isp.textMuted;
        statusText = 'Menunggu...';
        statusIcon = Icons.hourglass_empty_rounded;
    }

    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: isp.surfaceTertiary,
        borderRadius: BorderRadius.circular(IspRadii.lg),
        border: Border.all(
          color: isp.surfaceTertiary,
        ),
      ),
      child: Row(
        children: [
          Container(
            width: 48,
            height: 48,
            decoration: BoxDecoration(
              color: statusColor.withOpacity(0.12),
              borderRadius: BorderRadius.circular(12),
            ),
            child: Icon(icon, color: statusColor, size: 24),
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
                    fontSize: 15,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  description,
                  style: TextStyle(
                    color: isp.textMuted,
                    fontSize: 12,
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

/// Minimal i18n stub — uses hardcoded strings so we don't have a circular
/// import dependency. Extend app_localizations.dart for proper i18n.
class _L10n {
  _L10n._(this.context);
  final BuildContext context;

  String get continue_ => 'Lanjutkan';
  String get skip => 'Lewati Saja';
  String get notifications => 'Notifikasi Push';
  String get notificationsSub =>
      'Terima pengingat tagihan, update tiket, dan info gangguan.';
  String get camera => 'Kamera';
  String get cameraSub =>
      'Ambil foto untuk lampiran tiket dan profil avatar.';
}
