import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:ui_kit/ui_kit.dart';

void main() {
  testWidgets('IspErrorState shows message and retry button', (tester) async {
    var retried = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: IspErrorState(
          message: 'Gagal memuat',
          onRetry: () => retried++,
        ),
      ),
    );
    expect(find.text('Gagal memuat'), findsOneWidget);
    expect(find.text('Coba lagi'), findsOneWidget);
    await tester.tap(find.text('Coba lagi'));
    expect(retried, 1);
  });

  testWidgets('IspEmptyState renders icon + label + CTA', (tester) async {
    var tapped = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: IspEmptyState(
          message: 'Belum ada data',
          actionLabel: 'Tambah',
          onAction: () => tapped++,
        ),
      ),
    );
    expect(find.text('Belum ada data'), findsOneWidget);
    expect(find.text('Tambah'), findsOneWidget);
    await tester.tap(find.text('Tambah'));
    expect(tapped, 1);
  });
}
