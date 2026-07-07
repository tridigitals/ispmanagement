import 'package:flutter/material.dart';

enum StatusTone { neutral, success, warning, danger, info, primary }

/// Design tokens — dark theme matching the Tauri web app's palette
/// (see `src/lib/styles/global.css` in the Svelte frontend).
///
/// ⚠️ DEPRECATED — use `IspThemeColors` via `context.isp` instead.
/// Kept temporarily for `buildIspTheme()` compat; will be removed.
@Deprecated('Use IspThemeColors via context.isp')
class IspColors {
  IspColors._();
  static const Color primary = Color(0xFF7C4DFF);
  static const Color primaryHover = Color(0xFFB388FF);
  static const Color primaryLight = Color(0xFFCEBBFF);
  static const Color primarySubtle = Color(0x1F7C4DFF);
  static const Color success = Color(0xFF00E676);
  static const Color danger = Color(0xFFFF5252);
  static const Color warning = Color(0xFFFFD740);
  static const Color info = Color(0xFF40C4FF);
  static const Color bgApp = Color(0xFF060609);
  static const Color bgPrimary = Color(0xFF0D0F15);
  static const Color bgSecondary = Color(0xFF111119);
  static const Color bgSurface = Color(0xFF111119);
  static const Color bgTertiary = Color(0xFF1E1E28);
  static const Color bgHover = Color(0xFF242430);
  static const Color bgActive = Color(0xFF2A2A3A);
  static const Color textPrimary = Color(0xFFF0F0F5);
  static const Color textSecondary = Color(0xFF8888A0);
  static const Color textTertiary = Color(0xFF747D91);
  static const Color textMuted = Color(0xFF55556A);
  static const Color border = Color(0x291E1E2E);
  static const Color borderSubtle = Color(0x1A1E1E2E);
}

class IspRadii {
  IspRadii._();
  static const double sm = 6;
  static const double md = 8;
  static const double lg = 12;
  static const double xl = 16;
  static const double pill = 999;
}

class IspSpacing {
  IspSpacing._();
  static const double xs = 4;
  static const double sm = 8;
  static const double md = 12;
  static const double lg = 16;
  static const double xl = 24;
  static const double xxl = 32;
  static const double xxxl = 48;
}

class IspShadows {
  IspShadows._();
  static const List<BoxShadow> sm = [
    BoxShadow(color: Color(0x2E000000), blurRadius: 22, offset: Offset(0, 8)),
  ];
  static const List<BoxShadow> md = [
    BoxShadow(color: Color(0x33000000), blurRadius: 28, offset: Offset(0, 10)),
  ];
}

/// Dark Material 3 theme, tuned to match the Svelte app.
ThemeData buildIspTheme() {
  final base = ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: const ColorScheme.dark(
      primary: IspColors.primary,
      onPrimary: Color(0xFF0B0F1A),
      secondary: IspColors.primaryLight,
      onSecondary: Color(0xFF0B0F1A),
      surface: IspColors.bgSurface,
      onSurface: IspColors.textPrimary,
      surfaceContainerHighest: IspColors.bgTertiary,
      error: IspColors.danger,
      onError: Colors.white,
    ),
    scaffoldBackgroundColor: IspColors.bgApp,
    canvasColor: IspColors.bgApp,
  );

  return base.copyWith(
    textTheme: base.textTheme.apply(
      bodyColor: IspColors.textPrimary,
      displayColor: IspColors.textPrimary,
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: IspColors.bgApp,
      foregroundColor: IspColors.textPrimary,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
    ),
    cardTheme: CardTheme(
      color: IspColors.bgSurface,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(IspRadii.lg),
        side: const BorderSide(color: IspColors.borderSubtle),
      ),
    ),
    dividerTheme: const DividerThemeData(
      color: IspColors.borderSubtle,
      thickness: 1,
      space: 1,
    ),
    bottomNavigationBarTheme: const BottomNavigationBarThemeData(
      backgroundColor: IspColors.bgSecondary,
      selectedItemColor: IspColors.primary,
      unselectedItemColor: IspColors.textTertiary,
      type: BottomNavigationBarType.fixed,
      showSelectedLabels: true,
      showUnselectedLabels: true,
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: IspColors.bgSecondary,
      indicatorColor: IspColors.primarySubtle,
      labelTextStyle: WidgetStateProperty.resolveWith(
        (states) => TextStyle(
          fontSize: 12,
          fontWeight: states.contains(WidgetState.selected) ? FontWeight.w600 : FontWeight.w500,
          color: states.contains(WidgetState.selected)
              ? IspColors.primary
              : IspColors.textTertiary,
        ),
      ),
      iconTheme: WidgetStateProperty.resolveWith(
        (states) => IconThemeData(
          color: states.contains(WidgetState.selected)
              ? IspColors.primary
              : IspColors.textTertiary,
          size: 24,
        ),
      ),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: IspColors.bgTertiary,
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(IspRadii.md),
        borderSide: BorderSide.none,
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(IspRadii.md),
        borderSide: const BorderSide(color: IspColors.borderSubtle),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(IspRadii.md),
        borderSide: const BorderSide(color: IspColors.primary, width: 1.5),
      ),
      errorBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(IspRadii.md),
        borderSide: const BorderSide(color: IspColors.danger),
      ),
      labelStyle: const TextStyle(color: IspColors.textTertiary),
      hintStyle: const TextStyle(color: IspColors.textMuted),
    ),
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: IspColors.primary,
        foregroundColor: const Color(0xFF0B0F1A),
        elevation: 0,
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(IspRadii.md)),
        textStyle: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: IspColors.textPrimary,
        side: const BorderSide(color: IspColors.border),
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(IspRadii.md)),
        textStyle: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(foregroundColor: IspColors.primary),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: IspColors.bgTertiary,
      selectedColor: IspColors.primarySubtle,
      side: const BorderSide(color: IspColors.borderSubtle),
      labelStyle: const TextStyle(color: IspColors.textPrimary, fontSize: 12),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
    ),
  );
}
