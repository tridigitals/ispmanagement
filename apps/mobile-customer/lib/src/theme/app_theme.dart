import 'package:flutter/material.dart';
import 'package:ui_kit/ui_kit.dart';

/// Project theme. Wraps `ui_kit` with any app-specific overrides.
class AppTheme {
  AppTheme._();

  static ThemeData light() {
    return buildIspTheme().copyWith(
      brightness: Brightness.dark, // dark-only for now
    );
  }

  static ThemeData dark() => buildIspTheme();
}
