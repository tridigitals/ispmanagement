/// Form validation helpers. Keep them pure & lightweight — no Flutter import
/// so they can be unit-tested without a TestWidgetsFlutterBinding.
class Validators {
  Validators._();

  /// Indonesian phone: starts with 08, 10-13 digits total.
  static String? phone(String? value, {String? requiredMessage}) {
    if (value == null || value.trim().isEmpty) {
      return requiredMessage ?? 'Nomor HP wajib diisi';
    }
    final digits = value.replaceAll(RegExp(r'\D'), '');
    if (!RegExp(r'^08\d{8,11}$').hasMatch(digits)) {
      return 'Nomor HP tidak valid (contoh: 081234567890)';
    }
    return null;
  }

  /// Email — permissive but rejects obvious garbage.
  static String? email(String? value, {String? requiredMessage}) {
    if (value == null || value.trim().isEmpty) {
      return requiredMessage ?? 'Email wajib diisi';
    }
    final v = value.trim();
    final re = RegExp(r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$');
    if (!re.hasMatch(v)) return 'Email tidak valid';
    return null;
  }

  /// Password — min 8 chars, must contain a letter and a digit.
  static String? password(String? value, {String? requiredMessage}) {
    if (value == null || value.isEmpty) {
      return requiredMessage ?? 'Kata sandi wajib diisi';
    }
    if (value.length < 8) return 'Minimal 8 karakter';
    if (!value.contains(RegExp(r'[A-Za-z]'))) {
      return 'Harus mengandung huruf';
    }
    if (!value.contains(RegExp(r'\d'))) {
      return 'Harus mengandung angka';
    }
    return null;
  }

  /// OTP — exactly 6 digits.
  static String? otp(String? value, {String? requiredMessage, int length = 6}) {
    if (value == null || value.trim().isEmpty) {
      return requiredMessage ?? 'Kode OTP wajib diisi';
    }
    if (value.trim().length != length) {
      return 'Kode OTP harus $length digit';
    }
    if (!RegExp(r'^\d+$').hasMatch(value.trim())) {
      return 'Kode OTP hanya angka';
    }
    return null;
  }

  /// Indonesian invitation code: ISP-YYYY-XXXXX (10 chars + dashes).
  static String? inviteCode(String? value, {String? requiredMessage}) {
    if (value == null || value.trim().isEmpty) {
      return requiredMessage ?? 'Kode undangan wajib diisi';
    }
    final v = value.trim().toUpperCase();
    if (!RegExp(r'^ISP-\d{4}-[A-Z0-9]{5,10}$').hasMatch(v)) {
      return 'Format: ISP-YYYY-XXXXX';
    }
    return null;
  }

  /// Confirms two fields match (e.g. password & confirm password).
  static String? matches(
    String? value,
    String? otherValue, {
    String message = 'Tidak cocok dengan kolom sebelumnya',
  }) {
    if (value == null || value.isEmpty) return 'Wajib diisi';
    if (value != otherValue) return message;
    return null;
  }

  /// Required non-empty string (for short text fields).
  static String? required(String? value, {String? label}) {
    if (value == null || value.trim().isEmpty) {
      return '$label wajib diisi';
    }
    if (value.trim().length < 2) {
      return '$label minimal 2 karakter';
    }
    return null;
  }
}
