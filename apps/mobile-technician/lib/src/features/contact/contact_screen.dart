import 'package:flutter/material.dart';

class ContactScreen extends StatelessWidget {
  const ContactScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Hubungi Kami')),
      body: const Center(
        child: Padding(
          padding: EdgeInsets.all(32),
          child: Text(
            'Hubungi admin untuk bantuan.',
            style: TextStyle(fontSize: 14),
          ),
        ),
      ),
    );
  }
}
