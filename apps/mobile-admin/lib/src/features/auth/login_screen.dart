import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:mobile_admin/src/services/auth_providers.dart';

class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});

  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen> {
  final _identifierCtrl = TextEditingController();
  final _passwordCtrl = TextEditingController();
  bool _obscure = true;
  String _loginMethod = 'email'; // 'email' | 'phone'

  @override
  void dispose() {
    _identifierCtrl.dispose();
    _passwordCtrl.dispose();
    super.dispose();
  }

  void _login() {
    ref.read(authProvider.notifier).login(_identifierCtrl.text.trim(), _passwordCtrl.text);
  }

  @override
  Widget build(BuildContext context) {
    final auth = ref.watch(authProvider);

    return Scaffold(
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 400),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(Icons.admin_panel_settings, size: 64, color: Theme.of(context).colorScheme.primary),
                  const SizedBox(height: 16),
                  Text('ISP Admin', style: Theme.of(context).textTheme.headlineLarge),
                  const SizedBox(height: 8),
                  Text('Masuk ke panel admin', style: Theme.of(context).textTheme.bodyMedium),
                  const SizedBox(height: 40),

                  // Login method toggle
                  SegmentedButton<String>(
                    segments: [
                      ButtonSegment(
                        value: 'email',
                        label: const Text('Email'),
                        icon: const Icon(Icons.alternate_email, size: 18),
                      ),
                      ButtonSegment(
                        value: 'phone',
                        label: const Text('Phone'),
                        icon: const Icon(Icons.phone, size: 18),
                      ),
                    ],
                    selected: {_loginMethod},
                    onSelectionChanged: (Set<String> selection) {
                      setState(() {
                        _loginMethod = selection.first;
                        _identifierCtrl.clear();
                      });
                    },
                  ),
                  const SizedBox(height: 16),

                  TextField(
                    controller: _identifierCtrl,
                    keyboardType: _loginMethod == 'email'
                        ? TextInputType.emailAddress
                        : TextInputType.phone,
                    decoration: InputDecoration(
                      labelText: _loginMethod == 'email' ? 'Email' : 'Nomor HP',
                      prefixIcon: Icon(
                        _loginMethod == 'email' ? Icons.alternate_email : Icons.phone,
                      ),
                    ),
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: _passwordCtrl,
                    obscureText: _obscure,
                    decoration: InputDecoration(
                      labelText: 'Password',
                      prefixIcon: const Icon(Icons.lock_outline),
                      suffixIcon: IconButton(
                        icon: Icon(_obscure ? Icons.visibility_off : Icons.visibility),
                        onPressed: () => setState(() => _obscure = !_obscure),
                      ),
                    ),
                    onSubmitted: (_) => _login(),
                  ),
                  if (auth.error != null) ...[
                    const SizedBox(height: 12),
                    Text(auth.error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
                  ],
                  const SizedBox(height: 24),
                  SizedBox(
                    width: double.infinity,
                    child: ElevatedButton(
                      onPressed: auth.isLoading ? null : _login,
                      child: auth.isLoading
                          ? const SizedBox(height: 20, width: 20, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
                          : const Text('Masuk'),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
