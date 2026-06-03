/// Build-time configuration for the ISP mobile apps.
///
/// Edit values via `--dart-define` at build time, e.g.:
/// ```
/// flutter run --dart-define=API_BASE_URL=https://staging.example.com
/// ```
class BuildConfig {
  BuildConfig._();

  /// Base URL for the REST API.
  static const String apiBaseUrl = String.fromEnvironment(
    'API_BASE_URL',
    defaultValue: 'https://api-isp-management.tridigitals.com',
  );

  /// WebSocket base URL.
  static const String wsBaseUrl = String.fromEnvironment(
    'WS_BASE_URL',
    defaultValue: 'wss://api-isp-management.tridigitals.com',
  );

  /// Enable verbose logging (debug builds only).
  static const bool enableLogging = bool.fromEnvironment(
    'ENABLE_LOGGING',
    defaultValue: true,
  );

  /// Build flavor.
  static const String flavor = String.fromEnvironment('FLAVOR', defaultValue: 'prod');

  static bool get isProduction => flavor == 'prod';
  static bool get isStaging => flavor == 'staging';
  static bool get isDevelopment => flavor == 'dev';
}
