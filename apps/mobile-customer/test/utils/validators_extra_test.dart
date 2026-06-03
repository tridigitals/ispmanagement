import 'package:flutter_test/flutter_test.dart';
import 'package:mobile_customer/src/utils/form_validators.dart';

void main() {
  group('Validators.otp', () {
    test('accepts 6 digits', () {
      expect(Validators.otp('123456'), isNull);
    });
    test('rejects wrong length', () {
      expect(Validators.otp('12345'), isNotNull);
      expect(Validators.otp('1234567'), isNotNull);
    });
    test('rejects non-digits', () {
      expect(Validators.otp('abcdef'), isNotNull);
    });
  });

  group('Validators.matches', () {
    test('passes when equal', () {
      expect(Validators.matches('abc', 'abc'), isNull);
    });
    test('fails when different', () {
      expect(Validators.matches('abc', 'xyz'), isNotNull);
    });
    test('fails when empty', () {
      expect(Validators.matches('', 'abc'), isNotNull);
    });
  });

  group('Validators.required', () {
    test('uses label in error', () {
      expect(Validators.required('', label: 'Nama'), contains('Nama'));
    });
    test('passes valid', () {
      expect(Validators.required('Budi', label: 'Nama'), isNull);
    });
  });
}
