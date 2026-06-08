import 'package:flutter/material.dart';

/// Wraps the app with a Stack so that snackbars/toasts can be
/// shown by any descendant using [ScaffoldMessenger]. Most apps
/// don't need to customize this; it simply exists so the
/// [MaterialApp.builder] always has a sensible child.
class IspToastOverlay extends StatelessWidget {
  const IspToastOverlay({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return child;
  }
}
