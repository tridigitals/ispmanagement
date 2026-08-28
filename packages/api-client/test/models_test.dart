import 'package:api_client/api_client.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('SubscriptionModel', () {
    final sample = {
      'id': 'sub-1',
      'tenant_id': 't-1',
      'customer_id': 'c-1',
      'status': 'active',
      'billing_cycle': 'monthly',
      'price': 250000.0,
      'currency_code': 'IDR',
      'package_name': 'Paket 50Mbps',
      'starts_at': '2024-01-01T00:00:00Z',
    };

    test('parses JSON correctly', () {
      final s = SubscriptionModel.fromJson(sample);
      expect(s.id, 'sub-1');
      expect(s.isActive, true);
      expect(s.statusLabel(), 'Aktif');
    });

    test('flags needsAttention for suspended', () {
      final s = SubscriptionModel.fromJson({...sample, 'status': 'suspended'});
      expect(s.isSuspended, true);
      expect(s.needsAttention, true);
    });

    test('roundtrips toJson', () {
      final s = SubscriptionModel.fromJson(sample);
      final back = SubscriptionModel.fromJson(s.toJson());
      expect(back.id, s.id);
      expect(back.status, s.status);
    });
  });

  group('InvoiceModel', () {
    final base = {
      'id': 'inv-1',
      'invoice_number': 'INV/2024/001',
      'amount': 250000.0,
      'amount_paid': 0.0,
      'currency_code': 'IDR',
      'status': 'unpaid',
      'due_date': '2024-02-01T00:00:00Z',
      'created_at': '2024-01-01T00:00:00Z',
    };

    test('isOverdue when past due date', () {
      final inv = InvoiceModel.fromJson({
        ...base,
        'due_date': '2020-01-01T00:00:00Z',
      });
      expect(inv.isOverdue, true);
    });

    test('isPaid returns true for paid status', () {
      final inv = InvoiceModel.fromJson({...base, 'status': 'paid'});
      expect(inv.isPaid, true);
      expect(inv.amountRemaining, 0);
    });

    test('amountRemaining computes partial payment', () {
      final inv = InvoiceModel.fromJson({...base, 'amount_paid': 100000.0});
      expect(inv.amountRemaining, 150000.0);
    });
  });

  group('TicketModel', () {
    test('statusLabel matches Indonesian', () {
      final t = TicketModel.fromJson({
        'id': 't-1',
        'subject': 'Test',
        'status': 'in_progress',
        'priority': 'high',
        'created_at': '2024-01-01T00:00:00Z',
        'updated_at': '2024-01-02T00:00:00Z',
      });
      expect(t.statusLabel(), 'Ditangani');
      expect(t.priorityLabel(), 'Tinggi');
      expect(t.isOpen, true);
    });
  });

  group('UserModel', () {
    test('can() returns true for superadmin regardless of permissions', () {
      final u = UserModel(
        id: 'u-1',
        email: 'a@a.com',
        name: 'A',
        role: 'super_admin',
        isSuperAdmin: true,
      );
      expect(u.can('delete', 'tenant'), true);
    });

    test('can() returns true for matching permission', () {
      final u = UserModel(
        id: 'u-2',
        email: 'b@b.com',
        name: 'B',
        role: 'customer',
        isSuperAdmin: false,
        permissions: ['read:invoice'],
      );
      expect(u.can('read', 'invoice'), true);
      expect(u.can('delete', 'invoice'), false);
    });
  });

  group('ApiEndpoints', () {
    test('mySubscriptionById builds the portal endpoint', () {
      final out = ApiEndpoints.mySubscriptionById('abc');
      expect(out, '/api/customers/portal/my-subscriptions/abc');
    });
  });
}
