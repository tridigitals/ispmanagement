import 'dart:async';

import 'package:api_client/api_client.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'feature_providers.dart';
import 'service_providers.dart';
import 'settings_providers.dart';

// ── Data providers ──────────────────────────────────────────

final mySubscriptionsProvider =
    FutureProvider<List<SubscriptionModel>>((ref) async {
  final svc = ref.watch(subscriptionServiceProvider);
  final result = await svc.list();
  return result.getOrThrow().data;
});

final myInvoicesProvider = FutureProvider<List<InvoiceModel>>((ref) async {
  final svc = ref.watch(invoiceServiceProvider);
  final result = await svc.list();
  return result.getOrThrow().data;
});

final myTicketsProvider = FutureProvider<List<TicketModel>>((ref) async {
  final svc = ref.watch(ticketServiceProvider);
  final result = await svc.list();
  return result.getOrThrow().data;
});

/// Fetch a single invoice by ID.
final invoiceByIdProvider =
    FutureProvider.family<InvoiceModel, String>((ref, id) async {
  final svc = ref.watch(invoiceServiceProvider);
  final result = await svc.getById(id);
  return result.getOrThrow();
});

// ── Announcements provider ─────────────────────────────────

/// Auto-refreshes every 15 seconds so announcements stay realtime.
final activeAnnouncementsProvider =
    FutureProvider<List<AnnouncementModel>>((ref) async {
  // Keep alive so the timer runs continuously.
  ref.keepAlive();
  // Auto-invalidate every 15 seconds.
  final timer = Timer.periodic(const Duration(seconds: 15), (_) {
    ref.invalidateSelf();
  });
  ref.onDispose(timer.cancel);
  final svc = ref.watch(announcementServiceProvider);
  final result = await svc.getActive();
  return result.getOrThrow();
});

// ── Settings providers ──────────────────────────────────────

/// Biometric enabled flag — persisted to flutter_secure_storage.
class BiometricEnabledNotifier extends AsyncNotifier<bool> {
  @override
  Future<bool> build() async {
    final storage = ref.watch(secureStorageProvider);
    final val = await storage.read(key: 'biometric_enabled');
    return val == 'true';
  }

  Future<void> set(bool enabled) async {
    state = AsyncData(enabled);
    final storage = ref.read(secureStorageProvider);
    await storage.write(key: 'biometric_enabled', value: enabled.toString());
  }
}

final biometricEnabledProvider =
    AsyncNotifierProvider<BiometricEnabledNotifier, bool>(
  BiometricEnabledNotifier.new,
);

/// Persisted boolean notifier backed by SharedPreferences.
class PersistedBoolNotifier extends Notifier<bool> {
  PersistedBoolNotifier(this._key, this._defaultValue);
  final String _key;
  final bool _defaultValue;

  @override
  bool build() {
    final prefs = ref.read(sharedPreferencesProvider).valueOrNull;
    return prefs?.getBool(_key) ?? _defaultValue;
  }

  Future<void> set(bool value) async {
    state = value;
    final prefs = ref.read(sharedPreferencesProvider).valueOrNull;
    await prefs?.setBool(_key, value);
  }
}

final notifInvoiceEnabledProvider =
    NotifierProvider<PersistedBoolNotifier, bool>(
  () => PersistedBoolNotifier('notif_invoice', true),
);

final notifOutageEnabledProvider =
    NotifierProvider<PersistedBoolNotifier, bool>(
  () => PersistedBoolNotifier('notif_outage', true),
);

final notifPromoEnabledProvider = NotifierProvider<PersistedBoolNotifier, bool>(
  () => PersistedBoolNotifier('notif_promo', false),
);

final onboardingCompletedProvider = StateProvider<bool>((ref) => false);
