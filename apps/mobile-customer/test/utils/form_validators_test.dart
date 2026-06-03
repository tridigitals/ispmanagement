import 'package:flutter_test/flutter_test.dart';
import 'package:mobile_customer/src/utils/form_validators.dart';

void main() {
  group('Validators.phone', () {
    test('accepts valid 08 prefix', () {
      expect(Validators.phone('081234567890'), isNull);
      expect(Validators.phone('0812345678'), isNull);
    });
    test('rejects empty', () {
      expect(Validators.phone(''), isNotNull);
      expect(Validators.phone(null), isNotNull);
    });
    test('rejects non-08', () {
      expect(Validators.phone('+6281234567'), isNotNull);
      expect(Validators.phone('123456'), isNotNull);
    });
  });

  group('Validators.email', () {
    test('accepts valid emails', () {
      expect(Validators.email('user@example.com'), isNull);
      expect(Validators.email('a.b+c@sub.example.co.id'), isNull);
    });
    test('rejects invalid', () {
      expect(Validators.email('not-an-email'), isNotNull);
      expect(Validators.email('user@'), isNotNull);
    });
  });

  group('Validators.password', () {
    test('rejects short', () {
      expect(Validators.password('abc'), isNotNull);
    });
    test('rejects no digit', () {
      expect(Validators.password('abcdefgh'), isNotNull);
    });
    test('rejects no letter', () {
      expect(Validators.password('12345678'), isNotNull);
    });
    test('accepts strong', () {
      expect(Validators.password('Budi1234'), isNull);
    });
  });

  group('Validators.inviteCode', () {
    test('accepts ISP-YYYY-XXXX', () {
      expect(Validators.inviteCode('ISP-2024-ABCDE'), isNull);
    });
    test('rejects bad format', () {
      expect(Validators.inviteCode('ISP-24-ABC'), isNotNull);
      expect(Validators.inviteCode('XX-2024-ABCDE'), isNotNull);
    });
  });
}
