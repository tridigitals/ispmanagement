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

  testWidgets('IspListItem handles tap', (tester) async {
    var tapped = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: IspListItem(
            title: 'Item 1',
            subtitle: 'Sub',
            onTap: () => tapped++,
          ),
        ),
      ),
    );
    await tester.tap(find.text('Item 1'));
    expect(tapped, 1);
  });
}
