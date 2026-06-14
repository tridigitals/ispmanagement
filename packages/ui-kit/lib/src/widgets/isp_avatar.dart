import 'package:flutter/material.dart';

import '../theme/isp_theme_colors.dart';

/// Circle avatar with fallback to initials when no image is provided.
class IspAvatar extends StatelessWidget {
  const IspAvatar({
    super.key,
    this.imageUrl,
    this.initials,
    this.size = 40,
    this.backgroundColor,
  });

  final String? imageUrl;
  final String? initials;
  final double size;
  final Color? backgroundColor;

  @override
  Widget build(BuildContext context) {
    final isp = context.isp;
    final bgColor = backgroundColor ?? isp.accentSurface;

    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: bgColor,
        border: Border.all(color: isp.borderSubtle),
      ),
      clipBehavior: Clip.antiAlias,
      child: imageUrl != null && imageUrl!.isNotEmpty
          ? Image.network(
              imageUrl!,
              fit: BoxFit.cover,
              width: size,
              height: size,
              errorBuilder: (_, __, ___) => _fallback(bgColor, isp),
            )
          : _fallback(bgColor, isp),
    );
  }

  Widget _fallback(Color bgColor, IspThemeColors isp) {
    final text = (initials != null && initials!.isNotEmpty)
        ? initials!.substring(0, initials!.length.clamp(0, 2)).toUpperCase()
        : '?';
    return Center(
      child: Text(
        text,
        style: TextStyle(
          fontSize: size * 0.38,
          fontWeight: FontWeight.w600,
          color: isp.accent,
        ),
      ),
    );
  }
}
