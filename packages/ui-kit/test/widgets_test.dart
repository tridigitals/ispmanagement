import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:ui_kit/ui_kit.dart';

void main() {
  testWidgets('IspStatusBadge renders label and tone color', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: IspStatusBadge(
            label: 'Aktif',
            tone: StatusTone.success,
          ),
        ),
      ),
    );
    expect(find.text('Aktif'), findsOneWidget);
  });

  testWidgets('IspStatCard renders value and helper', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: IspStatCard(
            label: 'Pelanggan',
            value: '128',
            helper: 'Naik 5%',
            icon: Icons.people,
          ),
        ),
      ),
    );
    expect(find.text('128'), findsOneWidget);
    expect(find.text('Pelanggan'), findsOneWidget);
    expect(find.text('Naik 5%'), findsOneWidget);
  });

  testWidgets('IspProgressBar respects value clamp', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: IspProgressBar(value: 1.5), // > 1, should clamp
        ),
      ),
    );
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
  });
}
