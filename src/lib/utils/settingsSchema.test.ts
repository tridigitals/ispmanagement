import { describe, expect, it } from 'vitest';
import {
  SETTING_SECTIONS,
  initialValue,
  isVisible,
  schemaKeys,
  validate,
  type SettingField,
} from './settingsSchema';

const field = (over: Partial<SettingField> = {}): SettingField => ({
  key: 'k',
  label: 'K',
  type: 'text',
  ...over,
});

describe('initialValue', () => {
  it('memakai nilai server saat ada', () => {
    expect(initialValue(field({ fallback: 'x' }), 'dari-server')).toBe('dari-server');
  });

  it('memakai fallback saat server kosong', () => {
    /* Ini yang menggantikan 27 baris if-default di halaman lama. */
    expect(initialValue(field({ fallback: '60' }), '')).toBe('60');
    expect(initialValue(field({ fallback: '60' }), undefined)).toBe('60');
    expect(initialValue(field({ fallback: '60' }), '   ')).toBe('60');
  });

  it('mengembalikan string kosong kalau tidak ada fallback', () => {
    expect(initialValue(field(), undefined)).toBe('');
  });
});

describe('isVisible', () => {
  it('field tanpa syarat selalu terlihat', () => {
    expect(isVisible(field(), {})).toBe(true);
  });

  it('field bersyarat mengikuti nilai field acuan', () => {
    const f = field({ visibleWhen: { key: 'storage_driver', equals: 's3' } });
    expect(isVisible(f, { storage_driver: 's3' })).toBe(true);
    expect(isVisible(f, { storage_driver: 'system' })).toBe(false);
    expect(isVisible(f, {})).toBe(false);
  });
});

describe('schemaKeys', () => {
  it('tidak ada key ganda antar bagian', () => {
    /* Key ganda berarti satu pengaturan bisa disimpan dari dua tempat dengan
       nilai berbeda — persis kelas bug yang bikin halaman lama tidak bisa
       dipercaya. */
    const keys = schemaKeys();
    expect(new Set(keys).size).toBe(keys.length);
  });

  it('setiap field punya label non-kosong', () => {
    for (const s of SETTING_SECTIONS) {
      for (const f of s.fields) {
        expect(f.label.trim().length, `${s.id}/${f.key}`).toBeGreaterThan(0);
      }
    }
  });

  it('toggle selalu punya fallback supaya tidak pernah string kosong', () => {
    /* Toggle tanpa fallback berarti nilai awalnya '' yang bukan 'true' maupun
       'false'; tersimpan sebagai string kosong dan backend memperlakukannya
       sebagai tidak aktif tanpa pernah dinyatakan. */
    for (const s of SETTING_SECTIONS) {
      for (const f of s.fields.filter((x) => x.type === 'toggle')) {
        expect(['true', 'false'], `${s.id}/${f.key}`).toContain(f.fallback);
      }
    }
  });

  it('field bersyarat mengacu ke key yang benar-benar ada di skema', () => {
    const all = new Set(schemaKeys());
    for (const s of SETTING_SECTIONS) {
      for (const f of s.fields) {
        if (f.visibleWhen) {
          expect(all.has(f.visibleWhen.key), `${s.id}/${f.key} -> ${f.visibleWhen.key}`).toBe(true);
        }
      }
    }
  });

  it('rahasia tidak dirender sebagai teks biasa', () => {
    const secrets = SETTING_SECTIONS.flatMap((s) =>
      s.fields.filter((f) => /secret|password/.test(f.key)),
    );
    expect(secrets.length).toBeGreaterThan(0);
    for (const f of secrets) expect(f.type, f.key).toBe('password');
  });
});

describe('validate', () => {
  it('menolak SLA terlampaui yang lebih kecil dari SLA peringatan', () => {
    /* Halaman lama menutupi ini di pratinjau (warn x 2) tapi tetap menyimpan
       nilai aslinya. */
    const errors = validate({
      mikrotik_incident_sla_warn_minutes: '30',
      mikrotik_incident_sla_breach_minutes: '10',
    });
    expect(errors['mikrotik_incident_sla_breach_minutes']).toContain('30');
  });

  it('menerima SLA yang urut', () => {
    const errors = validate({
      mikrotik_incident_sla_warn_minutes: '30',
      mikrotik_incident_sla_breach_minutes: '120',
    });
    expect(errors['mikrotik_incident_sla_breach_minutes']).toBeUndefined();
  });

  it('menolak ambang CPU dan latensi yang terbalik', () => {
    const errors = validate({
      mikrotik_alert_cpu_risk: '85',
      mikrotik_alert_cpu_hot: '70',
      mikrotik_alert_latency_risk_ms: '400',
      mikrotik_alert_latency_hot_ms: '200',
    });
    expect(errors['mikrotik_alert_cpu_hot']).toBeTruthy();
    expect(errors['mikrotik_alert_latency_hot_ms']).toBeTruthy();
  });

  it('mewajibkan kredensial S3 hanya saat driver S3 dipilih', () => {
    expect(validate({ storage_driver: 'system' })['storage_s3_bucket']).toBeUndefined();

    const errors = validate({ storage_driver: 's3' });
    expect(errors['storage_s3_bucket']).toBeTruthy();
    expect(errors['storage_s3_access_key']).toBeTruthy();
    expect(errors['storage_s3_secret_key']).toBeTruthy();
  });

  it('nilai kosong tidak memicu galat perbandingan', () => {
    /* Field angka yang belum diisi tidak boleh dianggap salah urutan. */
    expect(Object.keys(validate({}))).toHaveLength(0);
  });
});
