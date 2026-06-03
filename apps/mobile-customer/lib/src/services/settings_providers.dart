import 'package:api_client/api_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'service_providers.dart';

/// Theme mode (light/dark/system) persisted in SharedPreferences.
final themeModeProvider = NotifierProvider<ThemeModeNotifier, ThemeMode>((ref) {
  return ThemeModeNotifier();
});

class ThemeModeNotifier extends Notifier<ThemeMode> {
  static const _kKey = 'theme_mode';
  late SharedPreferences _prefs;

  @override
  ThemeMode build() {
    _prefs = ref.read(sharedPreferencesProvider).requireValue;
    final stored = _prefs.getString(_kKey);
    return _parse(stored);
  }

  Future<void> set(ThemeMode mode) async {
    await _prefs.setString(_kKey, mode.name);
    state = mode;
  }

  ThemeMode _parse(String? value) {
    switch (value) {
      case 'light':
        return ThemeMode.light;
      case 'dark':
        return ThemeMode.dark;
      default:
        return ThemeMode.system;
    }
  }
}

/// Locale (id_ID / en_US) persisted in SharedPreferences.
final localeProvider = NotifierProvider<LocaleNotifier, Locale>((ref) {
  return LocaleNotifier();
});

class LocaleNotifier extends Notifier<Locale> {
  static const _kKey = 'locale';
  late SharedPreferences _prefs;

  @override
  Locale build() {
    _prefs = ref.read(sharedPreferencesProvider).requireValue;
    final stored = _prefs.getString(_kKey);
    return stored == 'en' ? const Locale('en') : const Locale('id');
  }

  Future<void> toggle() async {
    final next = state.languageCode == 'id' ? const Locale('en') : const Locale('id');
    await _prefs.setString(_kKey, next.languageCode);
    state = next;
  }
}

/// Async-loaded SharedPreferences (initialized in main()).
final sharedPreferencesProvider = FutureProvider<SharedPreferences>((ref) {
  return SharedPreferences.getInstance();
});
