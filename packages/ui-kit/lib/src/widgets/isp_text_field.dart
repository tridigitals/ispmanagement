import 'package:flutter/material.dart';

import '../theme/isp_theme_colors.dart';
import '../theme/theme.dart';

/// Themed text field with consistent Isp styling.
class IspTextField extends StatelessWidget {
  const IspTextField({
    super.key,
    this.controller,
    this.label,
    this.hint,
    this.prefixIcon,
    this.suffixIcon,
    this.errorText,
    this.obscureText = false,
    this.keyboardType,
    this.textInputAction,
    this.enabled = true,
    this.maxLines = 1,
    this.onChanged,
    this.onSubmitted,
    this.autofocus = false,
  });

  final TextEditingController? controller;
  final String? label;
  final String? hint;
  final IconData? prefixIcon;
  final Widget? suffixIcon;
  final String? errorText;
  final bool obscureText;
  final TextInputType? keyboardType;
  final TextInputAction? textInputAction;
  final bool enabled;
  final int? maxLines;
  final ValueChanged<String>? onChanged;
  final ValueChanged<String>? onSubmitted;
  final bool autofocus;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    return TextFormField(
      controller: controller,
      obscureText: obscureText,
      keyboardType: keyboardType,
      textInputAction: textInputAction,
      enabled: enabled,
      maxLines: maxLines,
      onChanged: onChanged,
      onFieldSubmitted: onSubmitted,
      autofocus: autofocus,
      style: TextStyle(fontSize: 15, color: isp.textPrimary),
      decoration: InputDecoration(
        labelText: label,
        hintText: hint,
        errorText: errorText,
        prefixIcon: prefixIcon != null
            ? Icon(prefixIcon, size: 20, color: isp.textMuted)
            : null,
        suffixIcon: suffixIcon,
        filled: true,
        fillColor: isp.surfaceTertiary,
        contentPadding: const EdgeInsets.symmetric(
          horizontal: IspSpacing.lg,
          vertical: IspSpacing.md + 2,
        ),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(IspRadii.md),
          borderSide: BorderSide.none,
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(IspRadii.md),
          borderSide: BorderSide(color: isp.borderSubtle),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(IspRadii.md),
          borderSide: BorderSide(color: isp.accent, width: 1.5),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(IspRadii.md),
          borderSide: BorderSide(color: isp.danger),
        ),
        focusedErrorBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(IspRadii.md),
          borderSide: BorderSide(color: isp.danger, width: 1.5),
        ),
        labelStyle: TextStyle(color: isp.textMuted),
        hintStyle: TextStyle(color: isp.textMuted),
        errorStyle: TextStyle(color: isp.danger, fontSize: 12),
      ),
    );
  }
}
