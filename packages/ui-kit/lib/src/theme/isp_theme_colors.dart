// ─── IspThemeColors — Theme-Aware Design Tokens ────────────────────
//
// Replaces both AppColors (mobile-customer) and IspColors (ui-kit).
// Access via `context.ispColors` extension helper, or directly:
//   Theme.of(context).extension<IspThemeColors>()!
//
// Color philosophy: premium, slate-based, derived from OKLCH-ish
// light/dark ramps. Dark mode is the default; light mode is fully
// production-ready with proper contrast on white backgrounds.
//
import 'package:flutter/material.dart';

/// Design tokens yang theme-aware. Pakai `context.isp` di widget
/// untuk mendapatkan nilai yang sesuai mode (light/dark).
@immutable
class IspThemeColors extends ThemeExtension<IspThemeColors> {
  // ─── Surface (backgrounds) ───────────────────────────────
  /// App scaffold background.
  final Color background;

  /// Card / panel background.
  final Color surface;

  /// Elevated card / modal / sheet.
  final Color surfaceElevated;

  /// Tertiary fill (input fields, chips, subtle containers).
  final Color surfaceTertiary;

  /// Hover/pressed overlay color.
  final Color surfaceHover;

  // ─── Border ─────────────────────────────────────────────
  /// Standard border, low contrast.
  final Color border;

  /// Subtle border, even lower contrast.
  final Color borderSubtle;

  /// Focus ring color (saturated accent).
  final Color borderFocus;

  // ─── Text ───────────────────────────────────────────────
  /// Primary body text.
  final Color textPrimary;

  /// Secondary text (captions, labels).
  final Color textSecondary;

  /// Muted text (placeholders, disabled).
  final Color textMuted;

  /// Inverse text (white on accent, dark on light bg).
  final Color textInverse;

  // ─── Brand ──────────────────────────────────────────────
  /// Primary brand accent (#6C5CE7 purple — Linear-style).
  final Color accent;

  /// Lighter accent variant (hover, secondary highlights).
  final Color accentLight;

  /// Subtle accent surface (15% in dark, 8% in light).
  final Color accentSurface;

  /// Accent text on accent surface.
  final Color accentText;

  // ─── Semantic status ────────────────────────────────────
  final Color success;
  final Color warning;
  final Color danger;
  final Color info;

  /// Subtle success surface (background tints).
  final Color successSurface;
  final Color warningSurface;
  final Color dangerSurface;
  final Color infoSurface;

  const IspThemeColors({
    required this.background,
    required this.surface,
    required this.surfaceElevated,
    required this.surfaceTertiary,
    required this.surfaceHover,
    required this.border,
    required this.borderSubtle,
    required this.borderFocus,
    required this.textPrimary,
    required this.textSecondary,
    required this.textMuted,
    required this.textInverse,
    required this.accent,
    required this.accentLight,
    required this.accentSurface,
    required this.accentText,
    required this.success,
    required this.warning,
    required this.danger,
    required this.info,
    required this.successSurface,
    required this.warningSurface,
    required this.dangerSurface,
    required this.infoSurface,
  });

  @override
  IspThemeColors copyWith({
    Color? background,
    Color? surface,
    Color? surfaceElevated,
    Color? surfaceTertiary,
    Color? surfaceHover,
    Color? border,
    Color? borderSubtle,
    Color? borderFocus,
    Color? textPrimary,
    Color? textSecondary,
    Color? textMuted,
    Color? textInverse,
    Color? accent,
    Color? accentLight,
    Color? accentSurface,
    Color? accentText,
    Color? success,
    Color? warning,
    Color? danger,
    Color? info,
    Color? successSurface,
    Color? warningSurface,
    Color? dangerSurface,
    Color? infoSurface,
  }) {
    return IspThemeColors(
      background: background ?? this.background,
      surface: surface ?? this.surface,
      surfaceElevated: surfaceElevated ?? this.surfaceElevated,
      surfaceTertiary: surfaceTertiary ?? this.surfaceTertiary,
      surfaceHover: surfaceHover ?? this.surfaceHover,
      border: border ?? this.border,
      borderSubtle: borderSubtle ?? this.borderSubtle,
      borderFocus: borderFocus ?? this.borderFocus,
      textPrimary: textPrimary ?? this.textPrimary,
      textSecondary: textSecondary ?? this.textSecondary,
      textMuted: textMuted ?? this.textMuted,
      textInverse: textInverse ?? this.textInverse,
      accent: accent ?? this.accent,
      accentLight: accentLight ?? this.accentLight,
      accentSurface: accentSurface ?? this.accentSurface,
      accentText: accentText ?? this.accentText,
      success: success ?? this.success,
      warning: warning ?? this.warning,
      danger: danger ?? this.danger,
      info: info ?? this.info,
      successSurface: successSurface ?? this.successSurface,
      warningSurface: warningSurface ?? this.warningSurface,
      dangerSurface: dangerSurface ?? this.dangerSurface,
      infoSurface: infoSurface ?? this.infoSurface,
    );
  }

  @override
  IspThemeColors lerp(ThemeExtension<IspThemeColors>? other, double t) {
    if (other is! IspThemeColors) return this;
    return IspThemeColors(
      background: Color.lerp(background, other.background, t)!,
      surface: Color.lerp(surface, other.surface, t)!,
      surfaceElevated:
          Color.lerp(surfaceElevated, other.surfaceElevated, t)!,
      surfaceTertiary:
          Color.lerp(surfaceTertiary, other.surfaceTertiary, t)!,
      surfaceHover: Color.lerp(surfaceHover, other.surfaceHover, t)!,
      border: Color.lerp(border, other.border, t)!,
      borderSubtle: Color.lerp(borderSubtle, other.borderSubtle, t)!,
      borderFocus: Color.lerp(borderFocus, other.borderFocus, t)!,
      textPrimary: Color.lerp(textPrimary, other.textPrimary, t)!,
      textSecondary: Color.lerp(textSecondary, other.textSecondary, t)!,
      textMuted: Color.lerp(textMuted, other.textMuted, t)!,
      textInverse: Color.lerp(textInverse, other.textInverse, t)!,
      accent: Color.lerp(accent, other.accent, t)!,
      accentLight: Color.lerp(accentLight, other.accentLight, t)!,
      accentSurface: Color.lerp(accentSurface, other.accentSurface, t)!,
      accentText: Color.lerp(accentText, other.accentText, t)!,
      success: Color.lerp(success, other.success, t)!,
      warning: Color.lerp(warning, other.warning, t)!,
      danger: Color.lerp(danger, other.danger, t)!,
      info: Color.lerp(info, other.info, t)!,
      successSurface:
          Color.lerp(successSurface, other.successSurface, t)!,
      warningSurface:
          Color.lerp(warningSurface, other.warningSurface, t)!,
      dangerSurface: Color.lerp(dangerSurface, other.dangerSurface, t)!,
      infoSurface: Color.lerp(infoSurface, other.infoSurface, t)!,
    );
  }

  // ─── Dark theme (default for ISP customer app) ─────────
  //
  // Philosophy: dark slate base, subtle elevation via lighter slate,
  // high contrast text, accent purple stays vivid.
  //
  static const dark = IspThemeColors(
    background: Color(0xFF0A0A0F),     // app bg
    surface: Color(0xFF141419),        // card
    surfaceElevated: Color(0xFF1C1C24), // elevated card / modal
    surfaceTertiary: Color(0xFF22222E), // input fill
    surfaceHover: Color(0xFF2A2A35),   // hover overlay
    border: Color(0xFF2A2A35),
    borderSubtle: Color(0xFF1F1F28),
    borderFocus: Color(0xFF6C5CE7),
    textPrimary: Color(0xFFF0F0F5),    // high contrast
    textSecondary: Color(0xFF8888A0),
    textMuted: Color(0xFF55556A),
    textInverse: Color(0xFF0A0A0F),    // dark on light
    accent: Color(0xFF6C5CE7),         // primary purple
    accentLight: Color(0xFFA29BFE),    // hover/secondary
    accentSurface: Color(0xFF2D2950),  // purple-15% (15% on dark bg)
    accentText: Color(0xFFA29BFE),
    success: Color(0xFF00D2A0),
    warning: Color(0xFFFFB347),
    danger: Color(0xFFFF6B6B),
    info: Color(0xFF4FC3F7),
    successSurface: Color(0xFF0E2E2A), // green-15%
    warningSurface: Color(0xFF3A2B14), // amber-15%
    dangerSurface: Color(0xFF3A1E20),  // red-15%
    infoSurface: Color(0xFF0F2A3A),    // blue-15%
  );

  // ─── Light theme ────────────────────────────────────────
  //
  // Philosophy: cool off-white bg, pure white cards, dark slate text.
  // Accent purple stays at #6C5CE7 (works on both backgrounds).
  // Surface variants use 8% (not 15%) because alpha blends stronger
  // on light backgrounds.
  //
  static const light = IspThemeColors(
    background: Color(0xFFF5F5FA),     // cool off-white
    surface: Colors.white,             // pure white card
    surfaceElevated: Color(0xFFFFFFFF), // same as surface
    surfaceTertiary: Color(0xFFEEEEF3), // input fill (gray-6%)
    surfaceHover: Color(0xFFE8E8EE),   // hover overlay (gray-9%)
    border: Color(0xFFE0E0EA),
    borderSubtle: Color(0xFFEEEEF3),
    borderFocus: Color(0xFF6C5CE7),
    textPrimary: Color(0xFF1A1A2E),    // near-black
    textSecondary: Color(0xFF6B6B80),
    textMuted: Color(0xFF9999AA),
    textInverse: Colors.white,
    accent: Color(0xFF6C5CE7),         // primary purple (same)
    accentLight: Color(0xFFA29BFE),    // hover (same)
    accentSurface: Color(0xFFEEEAFC),  // purple-6% on light
    accentText: Color(0xFF5847C9),     // darker purple for contrast
    success: Color(0xFF00A37A),        // slightly darker for AA contrast
    warning: Color(0xFFE08A1A),
    danger: Color(0xFFE64545),
    info: Color(0xFF1E89C7),
    successSurface: Color(0xFFE0F5EC), // green-6%
    warningSurface: Color(0xFFFDF1DE), // amber-6%
    dangerSurface: Color(0xFFFDE8E8),  // red-6%
    infoSurface: Color(0xFFE0F0FB),    // blue-6%
  );
}

/// BuildContext extension untuk akses ringkas ke design tokens.
/// Contoh: `final c = context.isp;` lalu `c.textPrimary`, `c.surface`.
extension IspColorsContext on BuildContext {
  IspThemeColors get isp {
    final ext = Theme.of(this).extension<IspThemeColors>();
    assert(ext != null,
        'IspThemeColors not registered. Ensure MaterialApp uses '
        'AppTheme.light()/dark() which register the extension.');
    return ext ?? IspThemeColors.dark;
  }
}
