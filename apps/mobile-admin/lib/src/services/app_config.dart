import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

class AppConfig {
  final String apiBaseUrl;
  final bool isDemoMode;

  const AppConfig({
    this.apiBaseUrl = 'http://103.190.112.214:3000/api',
    this.isDemoMode = false,
  });
}

final appConfigProvider = Provider<AppConfig>((ref) {
  return const AppConfig();
});

final sharedPrefsProvider = FutureProvider<SharedPreferences>((ref) {
  return SharedPreferences.getInstance();
});
