import 'package:flutter/material.dart';
import 'isp_theme_colors.dart';

/// Neubrutalist design system — matches mockup tokens.
/// 
/// Card formula: `border: 1.5px solid`, `boxShadow: 3px 3px 0`, active `translate(1,1)`
class NbStyle {
  NbStyle._();

  /// Neubrutalist card decoration.
  /// Usage: `Container(decoration: NbStyle.card(context), ...)`
  static BoxDecoration card(BuildContext context, {BorderRadius? radius}) {
    final isp = IspThemeColors.of(context);
    return BoxDecoration(
      color: isp.surface,
      border: Border.all(color: isp.border, width: 1.5),
      borderRadius: radius ?? BorderRadius.circular(8),
      boxShadow: [
        BoxShadow(color: isp.border.withOpacity(0.5), offset: const Offset(3, 3), blurRadius: 0),
      ],
    );
  }

  /// Elevated button with neubrutalist shadow — solid accent color (no gradient).
  static ButtonStyle accentButton(BuildContext context, {bool outline = false}) {
    final isp = IspThemeColors.of(context);
    return ButtonStyle(
      elevation: WidgetStateProperty.all(0),
      backgroundColor: WidgetStateProperty.all(outline ? Colors.transparent : isp.accent),
      foregroundColor: WidgetStateProperty.all(outline ? isp.accent : Colors.white),
      shape: WidgetStateProperty.all(
        RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
          side: BorderSide(color: isp.accent, width: 1.5),
        ),
      ),
      padding: WidgetStateProperty.all(const EdgeInsets.symmetric(vertical: 14, horizontal: 24)),
    );
  }

  /// Input field decoration — pill-shaped with 1.5px border.
  /// Usage: `TextField(decoration: NbStyle.inputField(context, label: 'Email'))`
  static InputDecoration inputField(BuildContext context, {String? label, String? hint, Widget? prefix, Widget? suffix}) {
    final isp = IspThemeColors.of(context);
    return InputDecoration(
      labelText: label,
      hintText: hint,
      prefixIcon: prefix,
      suffixIcon: suffix,
      filled: true,
      fillColor: isp.surface,
      labelStyle: TextStyle(color: isp.textSecondary, fontSize: 13),
      hintStyle: TextStyle(color: isp.textMuted, fontSize: 13),
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: BorderSide(color: isp.border, width: 1.5),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: BorderSide(color: isp.border, width: 1.5),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: BorderSide(color: isp.accent, width: 1.5),
      ),
      errorBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: const BorderSide(color: Color(0xFFFF5252), width: 1.5),
      ),
    );
  }

  /// Tinted icon container (Apple settings style) — 32×32, 7px radius, tinted bg.
  /// Usage: `NbStyle.iconContainer(context, Icons.wifi, color: isp.accent)`
  static Container iconContainer(BuildContext context, IconData icon, {Color? color, double size = 18}) {
    final isp = IspThemeColors.of(context);
    final tint = color ?? isp.accent;
    return Container(
      width: 34,
      height: 34,
      decoration: BoxDecoration(
        color: tint.withOpacity(0.15),
        borderRadius: BorderRadius.circular(7),
      ),
      alignment: Alignment.center,
      child: Icon(icon, size: size, color: tint),
    );
  }

  /// Status pill badge — neubrutalist small label with severity background.
  static Container statusPill(BuildContext context, String label, {Color? bgColor, Color? textColor}) {
    final isp = IspThemeColors.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: (bgColor ?? isp.success).withOpacity(0.15),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: (bgColor ?? isp.success).withOpacity(0.3), width: 1),
      ),
      child: Text(label, style: TextStyle(color: textColor ?? bgColor ?? isp.success, fontSize: 11, fontWeight: FontWeight.w600)),
    );
  }

  /// Animated press effect — scale down + translate for neubrutalist "push" feel.
  static Widget pressable(Widget child, {VoidCallback? onTap}) {
    return _NeubrutalistPressable(onTap: onTap, child: child);
  }
}

class _NeubrutalistPressable extends StatefulWidget {
  final Widget child;
  final VoidCallback? onTap;
  const _NeubrutalistPressable({this.onTap, required this.child});

  @override
  State<_NeubrutalistPressable> createState() => _NeubrutalistPressableState();
}

class _NeubrutalistPressableState extends State<_NeubrutalistPressable> {
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTapDown: (_) => setState(() => _pressed = true),
      onTapUp: (_) {
        setState(() => _pressed = false);
        widget.onTap?.call();
      },
      onTapCancel: () => setState(() => _pressed = false),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 80),
        transform: _pressed ? Matrix4.translationValues(1, 1, 0) : Matrix4.identity,
        child: widget.child,
      ),
    );
  }
}
