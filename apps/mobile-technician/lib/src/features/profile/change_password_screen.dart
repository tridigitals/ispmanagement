import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:ui_kit/ui_kit.dart';
import '../../services/auth_providers.dart';

class ChangePasswordScreen extends ConsumerStatefulWidget {
  const ChangePasswordScreen({super.key});
  @override
  ConsumerState<ChangePasswordScreen> createState() => _ChangePasswordScreenState();
}

class _ChangePasswordScreenState extends ConsumerState<ChangePasswordScreen> {
  final _formKey = GlobalKey<FormState>();
  final _currentCtrl = TextEditingController();
  final _newCtrl = TextEditingController();
  final _confirmCtrl = TextEditingController();
  bool _saving = false;
  bool _obscure = true;

  @override
  void dispose() {
    _currentCtrl.dispose();
    _newCtrl.dispose();
    _confirmCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Ubah Password')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Form(
          key: _formKey,
          child: Column(
            children: [
              TextFormField(
                controller: _currentCtrl,
                obscureText: _obscure,
                decoration: InputDecoration(
                  labelText: 'Password Saat Ini',
                  suffixIcon: IconButton(
                    icon: Icon(_obscure ? Icons.visibility : Icons.visibility_off),
                    onPressed: () => setState(() => _obscure = !_obscure),
                  ),
                ),
                validator: (v) => (v == null || v.length < 6) ? 'Minimal 6 karakter' : null,
              ),
              const SizedBox(height: 12),
              TextFormField(
                controller: _newCtrl,
                obscureText: true,
                decoration: const InputDecoration(labelText: 'Password Baru'),
                validator: (v) => (v == null || v.length < 6) ? 'Minimal 6 karakter' : null,
              ),
              const SizedBox(height: 12),
              TextFormField(
                controller: _confirmCtrl,
                obscureText: true,
                decoration: const InputDecoration(labelText: 'Konfirmasi Password Baru'),
                validator: (v) => v != _newCtrl.text ? 'Password tidak cocok' : null,
              ),
              const SizedBox(height: 24),
              ElevatedButton(
                onPressed: _saving
                    ? null
                    : () async {
                        if (!_formKey.currentState!.validate()) return;
                        setState(() => _saving = true);
                        final res = await ref
                            .read(authControllerProvider.notifier)
                            .changePassword(
                              current: _currentCtrl.text,
                              next: _newCtrl.text,
                            );
                        setState(() => _saving = false);
                        if (mounted && res is Success) {
                          Navigator.pop(context);
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(content: Text('Password berhasil diubah')),
                          );
                        } else if (mounted && res is Failure) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text((res as Failure).exception.message)),
                          );
                        }
                      },
                child: _saving
                    ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))
                    : const Text('Ubah Password'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
