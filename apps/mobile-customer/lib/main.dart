import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'src/app.dart';
import 'src/router/app_router.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // Pre-init the prefs so the router can read the onboarding flag
  // synchronously on first build.
  await SharedPreferences.getInstance();
  runApp(const ProviderScope(child: IspCustomerApp()));
}
