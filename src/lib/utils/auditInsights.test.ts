import { describe, expect, it } from 'vitest';
import {
  actionTone,
  describeActor,
  resourceLabel,
  summarizeDetails,
  toIsoRange,
  validateDateRange,
  type AuditLogRow,
} from './auditInsights';

function row(over: Partial<AuditLogRow> = {}): AuditLogRow {
  return {
    id: 'a1',
    user_id: null,
    tenant_id: 't1',
    action: 'update',
    resource: 'settings',
    resource_id: 'r1',
    details: null,
    ip_address: null,
    created_at: '2026-09-04T00:00:00Z',
    ...over,
  };
}

describe('describeActor', () => {
  it('user aktif dengan nama+email tampil lengkap', () => {
    const a = describeActor(row({ user_id: 'u1', user_name: 'Tri', user_email: 't@x.id' }));
    expect(a.kind).toBe('user');
    expect(a.label).toBe('Tri — t@x.id');
  });

  it('user tanpa nama tapi dengan email tetap user', () => {
    const a = describeActor(row({ user_id: 'u1', user_email: 't@x.id' }));
    expect(a.kind).toBe('user');
    expect(a.label).toBe('t@x.id');
  });

  it('user_id NULL = aksi sistem, bukan strip kosong', () => {
    const a = describeActor(row({ user_id: null }));
    expect(a.kind).toBe('system');
    expect(a.label).toBe('Sistem');
    // Regression: UI lama menampilkan "—" yang tidak bisa dibedakan.
    expect(a.label).not.toBe('—');
  });

  it('user_id terisi tapi join miss = user terhapus, dengan id di detail', () => {
    const a = describeActor(row({ user_id: 'dead-uuid' }));
    expect(a.kind).toBe('deleted');
    expect(a.label).toBe('User terhapus');
    expect(a.detail).toContain('dead-uuid');
  });
});

describe('summarizeDetails', () => {
  it('null dan string kosong = empty', () => {
    expect(summarizeDetails(null).kind).toBe('empty');
    expect(summarizeDetails('   ').kind).toBe('empty');
  });

  it('teks bebas (1.000 baris di produksi) lewat utuh sebagai text', () => {
    const s = summarizeDetails('Latency alert: 955ms on Xtrabit');
    expect(s.kind).toBe('text');
    expect(s.summary).toBe('Latency alert: 955ms on Xtrabit');
  });

  it('JSON objek diringkas key=value dan field terurai', () => {
    const s = summarizeDetails('{"amount":50000,"status":"paid"}');
    expect(s.kind).toBe('json');
    expect(s.summary).toContain('amount=50000');
    expect(s.summary).toContain('status=paid');
    expect(s.fields).toEqual([
      { key: 'amount', value: '50000' },
      { key: 'status', value: 'paid' },
    ]);
  });

  it('JSON rusak (terpotong) TIDAK melempar dan tidak jadi "{}" kosong', () => {
    const s = summarizeDetails('{"pecah": true');
    expect(s.kind).toBe('text');
    expect(s.summary).toContain('pecah');
  });

  it('JSON array diperlakukan teks, bukan fields kosong', () => {
    const s = summarizeDetails('[1,2,3]');
    expect(s.kind).toBe('text');
    expect(s.fields).toEqual([]);
  });

  it('JSON null scalar tidak dianggap objek', () => {
    expect(summarizeDetails('null').kind).toBe('text');
  });
});

describe('resourceLabel', () => {
  it('memetakan kode produksi ke label Indonesia', () => {
    expect(resourceLabel('billing')).toBe('Penagihan');
    expect(resourceLabel('mikrotik_alert')).toBe('Peringatan router');
    expect(resourceLabel('customer_subscriptions')).toBe('Langganan');
  });

  it('kode tak dikenal lewat apa adanya (bukan crash/kosong)', () => {
    expect(resourceLabel('mesin_baru')).toBe('mesin_baru');
  });
});

describe('actionTone', () => {
  it('kegagalan & penghapusan merah', () => {
    expect(actionTone('login_failed')).toBe('negative');
    expect(actionTone('login_locked')).toBe('negative');
    expect(actionTone('delete')).toBe('negative');
  });

  it('peringatan & suspensi kuning', () => {
    expect(actionTone('alert_latency')).toBe('warning');
    expect(actionTone('status_offline')).toBe('warning');
    expect(actionTone('suspend')).toBe('warning');
  });

  it('pembuatan & penerbitan hijau', () => {
    expect(actionTone('create')).toBe('positive');
    expect(actionTone('publish')).toBe('positive');
    expect(actionTone('USER_REGISTER')).toBe('positive');
    expect(actionTone('status_online')).toBe('positive');
  });

  it('update dan keluarga bertitik netral', () => {
    expect(actionTone('update')).toBe('neutral');
    expect(actionTone('billing.collection_run')).toBe('neutral');
  });
});

describe('toIsoRange', () => {
  it('tanggal-only: mulai tengah malam lokal, akhir 23:59:59.999 lokal', () => {
    const r = toIsoRange('2026-09-01', '2026-09-03');
    // Dites lewat komponen lokal (bukan string UTC) supaya tidak bergantung
    // zona waktu mesin — yang diuji adalah CAKUPAN HARI, bukan offset.
    const a = new Date(r.date_from!);
    const b = new Date(r.date_to!);
    expect([a.getMonth() + 1, a.getDate(), a.getHours(), a.getMinutes()]).toEqual([9, 1, 0, 0]);
    // Regression: `<= 00:00:00` lama membuat hari terakhir selalu kosong.
    expect([b.getMonth() + 1, b.getDate(), b.getHours(), b.getMinutes()]).toEqual([9, 3, 23, 59]);
    expect(b.getSeconds()).toBe(59);
  });

  it('datetime-local dengan jam lewat apa adanya', () => {
    const r = toIsoRange('2026-09-01T07:30', '2026-09-03T18:45');
    // Jam yang sama harus bertahan saat dibaca kembali sebagai waktu lokal.
    const a = new Date(r.date_from!);
    const b = new Date(r.date_to!);
    expect([a.getDate(), a.getHours(), a.getMinutes()]).toEqual([1, 7, 30]);
    expect([b.getDate(), b.getHours(), b.getMinutes()]).toEqual([3, 18, 45]);
  });

  it('filter kosong tidak mengirim kunci', () => {
    expect(toIsoRange('', '')).toEqual({});
    expect(toIsoRange('2026-09-01', '')).toEqual({ date_from: expect.any(String) });
  });
});

describe('validateDateRange', () => {
  it('rentang sah = null', () => {
    expect(validateDateRange('2026-09-01', '2026-09-03')).toBeNull();
    expect(validateDateRange('', '2026-09-03')).toBeNull();
  });

  it('mulai setelah akhir ditolak sebelum request', () => {
    expect(validateDateRange('2026-09-03', '2026-09-01')).toMatch('tidak boleh');
  });

  it('tanggal tak terbaca ditolak', () => {
    expect(validateDateRange('kemarin', '2026-09-03')).toMatch('dapat dibaca');
  });
});
