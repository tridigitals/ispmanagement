import { describe, it, expect } from 'vitest';
import type { Announcement } from '$lib/api/types';
import {
  alreadyDelivered,
  announcementStatus,
  audienceOptions,
  bodyExcerpt,
  deliveryLabels,
  editDeliveryWarning,
  parseWaktu,
  portalCoverageGap,
  scopeLabel,
  severityTone,
  statusCounts,
  statusLabel,
  statusTone,
  toIso,
  validateDraft,
  type DraftInput,
} from './announcementInsights';

/**
 * Angka acuan dari data produksi tenant ISP Management (2026-09-04):
 * 16 pengumuman, semuanya `notified_at` terisi; 2 tayang, 0 terjadwal,
 * 14 kedaluwarsa; 548 pelanggan tapi hanya 2 punya akun portal;
 * jangkauan audiens: admins 1, customers 2, active_subscribers 0,
 * suspended_subscribers 1, all 6; global mengirim ke 18 user aktif.
 */
const SEKARANG = new Date('2026-09-04T12:00:00Z').getTime();

function buatAnnouncement(patch: Partial<Announcement> = {}): Announcement {
  return {
    id: 'a1',
    tenant_id: 'f4ba7f24-ce3c-4f87-b3f2-b5e7d7977c74',
    created_by: 'u1',
    cover_file_id: null,
    title: 'Pemeliharaan jaringan',
    body: '<p>Akan ada pemeliharaan</p>',
    severity: 'info',
    audience: 'all',
    mode: 'post',
    format: 'html',
    deliver_in_app: true,
    deliver_email: false,
    deliver_email_force: true,
    starts_at: '2026-09-01T00:00:00Z',
    ends_at: null,
    notified_at: '2026-09-01T00:05:00Z',
    created_at: '2026-09-01T00:00:00Z',
    updated_at: '2026-09-01T00:00:00Z',
    ...patch,
  };
}

describe('announcementStatus — sepakat dengan SQL server', () => {
  it('menandai tayang saat sudah mulai dan tanpa tanggal akhir', () => {
    const a = buatAnnouncement({ starts_at: '2026-09-01T00:00:00Z', ends_at: null });
    expect(announcementStatus(a, SEKARANG)).toBe('active');
  });

  it('menandai tayang saat berada di dalam rentang', () => {
    const a = buatAnnouncement({
      starts_at: '2026-09-01T00:00:00Z',
      ends_at: '2026-09-30T00:00:00Z',
    });
    expect(announcementStatus(a, SEKARANG)).toBe('active');
  });

  it('menandai terjadwal saat mulai masih di depan', () => {
    const a = buatAnnouncement({ starts_at: '2026-09-10T00:00:00Z' });
    expect(announcementStatus(a, SEKARANG)).toBe('scheduled');
  });

  it('menandai kedaluwarsa saat tanggal akhir sudah lewat', () => {
    const a = buatAnnouncement({
      starts_at: '2026-08-01T00:00:00Z',
      ends_at: '2026-08-31T00:00:00Z',
    });
    expect(announcementStatus(a, SEKARANG)).toBe('expired');
  });

  it('kedaluwarsa menang atas terjadwal — mengikuti urutan cek di SQL', () => {
    // Baris rusak (akhir sebelum mulai) tidak mungkin lewat validasi server,
    // tapi kalau ada, klien harus menjawab sama seperti SQL: expired.
    const a = buatAnnouncement({
      starts_at: '2026-09-20T00:00:00Z',
      ends_at: '2026-08-01T00:00:00Z',
    });
    expect(announcementStatus(a, SEKARANG)).toBe('expired');
  });

  it('memperlakukan ends_at null sebagai tanpa batas, bukan epoch 0', () => {
    const a = buatAnnouncement({ starts_at: '2020-01-01T00:00:00Z', ends_at: null });
    expect(announcementStatus(a, SEKARANG)).toBe('active');
  });

  it('menghitung rekap status', () => {
    const rows = [
      buatAnnouncement({ id: '1', starts_at: '2026-09-01T00:00:00Z', ends_at: null }),
      buatAnnouncement({ id: '2', starts_at: '2026-09-10T00:00:00Z', ends_at: null }),
      buatAnnouncement({
        id: '3',
        starts_at: '2026-08-01T00:00:00Z',
        ends_at: '2026-08-15T00:00:00Z',
      }),
      buatAnnouncement({
        id: '4',
        starts_at: '2026-08-01T00:00:00Z',
        ends_at: '2026-08-20T00:00:00Z',
      }),
    ];
    expect(statusCounts(rows, SEKARANG)).toEqual({ active: 1, scheduled: 1, expired: 2 });
  });
});

describe('peringatan penyuntingan pengumuman terkirim', () => {
  it('mengenali pengumuman yang sudah dikirim', () => {
    expect(alreadyDelivered(buatAnnouncement({ notified_at: '2026-09-01T00:05:00Z' }))).toBe(true);
    expect(alreadyDelivered(buatAnnouncement({ notified_at: null }))).toBe(false);
  });

  it('memperingatkan bahwa penyuntingan tidak mengirim ulang', () => {
    // Dibuktikan di DB: menggeser starts_at pengumuman terkirim tetap
    // menghasilkan 0 baris terpilih oleh penjadwal, karena UPDATE tidak
    // pernah mereset notified_at.
    const pesan = editDeliveryWarning(buatAnnouncement({ notified_at: '2026-09-01T00:05:00Z' }));
    expect(pesan).toBeTruthy();
    expect(pesan).toContain('tidak mengirim notifikasi baru');
  });

  it('tidak memperingatkan pengumuman yang belum terkirim', () => {
    expect(editDeliveryWarning(buatAnnouncement({ notified_at: null }))).toBeNull();
  });
});

describe('audienceOptions — jangkauan nyata, bukan hanya label', () => {
  const jangkauanTenant = {
    all: 6,
    admins: 1,
    customers: 2,
    active_subscribers: 0,
    suspended_subscribers: 1,
  };

  it('menyertakan jumlah penerima per pilihan', () => {
    const opsi = audienceOptions(jangkauanTenant);
    expect(opsi.find((o) => o.value === 'all')?.recipients).toBe(6);
    expect(opsi.find((o) => o.value === 'admins')?.recipients).toBe(1);
    expect(opsi.find((o) => o.value === 'customers')?.recipients).toBe(2);
  });

  it('memperingatkan pilihan yang menjangkau nol akun', () => {
    const opsi = audienceOptions(jangkauanTenant);
    const nol = opsi.find((o) => o.value === 'active_subscribers');
    expect(nol?.recipients).toBe(0);
    expect(nol?.warning).toContain('tidak akan sampai');
  });

  it('tidak memperingatkan pilihan yang punya penerima', () => {
    const opsi = audienceOptions(jangkauanTenant);
    expect(opsi.find((o) => o.value === 'customers')?.warning).toBeNull();
  });

  it('tetap menampilkan semua pilihan meski nol — masalahnya tidak disembunyikan', () => {
    expect(audienceOptions(jangkauanTenant)).toHaveLength(5);
  });

  it('scope tidak lagi mengubah hasil — cabang global sudah diperbaiki di Rust', () => {
    // Dulu global mengirim ke semua user aktif dan mengabaikan audiens.
    // Setelah `global_recipient_ids()`, pilihan yang sama-sama nol tetap
    // diperingatkan di kedua scope.
    const opsi = audienceOptions(jangkauanTenant);
    expect(opsi.find((o) => o.value === 'active_subscribers')?.warning).toContain(
      'tidak akan sampai'
    );
    expect(opsi.find((o) => o.value === 'admins')?.warning).toBeNull();
  });

  it('mengembalikan null saat jangkauan belum dihitung', () => {
    const opsi = audienceOptions({});
    expect(opsi.every((o) => o.recipients === null)).toBe(true);
  });
});

describe('portalCoverageGap', () => {
  it('menjelaskan kesenjangan akun portal di data nyata', () => {
    const pesan = portalCoverageGap(548, 2);
    expect(pesan).toContain('2 dari 548');
    expect(pesan).toContain('0%');
  });

  it('diam saat semua pelanggan punya akun', () => {
    expect(portalCoverageGap(10, 10)).toBeNull();
    expect(portalCoverageGap(10, 12)).toBeNull();
  });

  it('diam saat belum ada pelanggan', () => {
    expect(portalCoverageGap(0, 0)).toBeNull();
  });
});

describe('validateDraft — menahan galat sebelum cover diunggah', () => {
  function draf(patch: Partial<DraftInput> = {}): DraftInput {
    return {
      title: 'Judul',
      body: '<p>isi</p>',
      startsAt: '',
      endsAt: '',
      deliverInApp: true,
      deliverEmail: false,
      scope: 'tenant',
      ...patch,
    };
  }

  it('menerima draf yang sah', () => {
    expect(validateDraft(draf())).toEqual([]);
  });

  it('menolak judul kosong', () => {
    expect(validateDraft(draf({ title: '   ' }))).toEqual([
      { field: 'title', message: 'Judul wajib diisi.' },
    ]);
  });

  it('menolak isi yang hanya markup', () => {
    const masalah = validateDraft(draf({ body: '<p></p>' }));
    expect(masalah.map((m) => m.field)).toContain('body');
  });

  it('menolak draf tanpa kanal pengiriman', () => {
    const masalah = validateDraft(draf({ deliverInApp: false, deliverEmail: false }));
    expect(masalah.map((m) => m.field)).toContain('delivery');
  });

  it('menolak akhir sebelum mulai — aturan yang sama dengan server', () => {
    const masalah = validateDraft(
      draf({ startsAt: '2026-09-10T10:00', endsAt: '2026-09-09T10:00' })
    );
    expect(masalah.map((m) => m.field)).toContain('endsAt');
  });

  it('menolak akhir yang sama dengan mulai', () => {
    const masalah = validateDraft(
      draf({ startsAt: '2026-09-10T10:00', endsAt: '2026-09-10T10:00' })
    );
    expect(masalah.map((m) => m.field)).toContain('endsAt');
  });

  it('menolak tanggal yang tak bisa dibaca alih-alih menerbitkan sekarang', () => {
    // Perilaku lama: toIsoOrNull mengembalikan null → server memaknai now().
    const masalah = validateDraft(draf({ startsAt: 'besok pagi' }));
    expect(masalah.map((m) => m.field)).toContain('startsAt');
  });

  it('menerima tanggal kosong sebagai terbit sekarang', () => {
    expect(validateDraft(draf({ startsAt: '', endsAt: '' }))).toEqual([]);
  });
});

describe('parseWaktu & toIso', () => {
  it('membedakan kosong dari tak valid', () => {
    expect(parseWaktu('')).toBeNull();
    expect(parseWaktu('   ')).toBeNull();
    expect(parseWaktu('bukan tanggal')).toBeNull();
    expect(parseWaktu('2026-09-10T10:00')).toBeTypeOf('number');
  });

  it('mengubah nilai sah menjadi ISO', () => {
    expect(toIso('2026-09-10T10:00:00Z')).toBe('2026-09-10T10:00:00.000Z');
  });

  it('mengembalikan null untuk masukan tak valid', () => {
    expect(toIso('kapan-kapan')).toBeNull();
    expect(toIso('')).toBeNull();
  });
});

describe('label tampilan', () => {
  it('memetakan severity ke nada badge', () => {
    expect(severityTone('success')).toBe('positive');
    expect(severityTone('warning')).toBe('warning');
    expect(severityTone('error')).toBe('negative');
    expect(severityTone('info')).toBe('neutral');
    expect(severityTone('entah')).toBe('neutral');
  });

  it('memetakan status ke nada dan label', () => {
    expect(statusTone('active')).toBe('positive');
    expect(statusTone('scheduled')).toBe('warning');
    expect(statusTone('expired')).toBe('neutral');
    expect(statusLabel('active')).toBe('Tayang');
    expect(statusLabel('scheduled')).toBe('Terjadwal');
    expect(statusLabel('expired')).toBe('Kedaluwarsa');
  });

  it('menandai scope', () => {
    expect(scopeLabel(buatAnnouncement({ tenant_id: null }))).toBe('Global');
    expect(scopeLabel(buatAnnouncement({ tenant_id: 'abc' }))).toBe('Tenant');
  });

  it('menyebut kanal yang aktif', () => {
    expect(deliveryLabels(buatAnnouncement({ deliver_in_app: true, deliver_email: false }))).toEqual(
      ['Notifikasi aplikasi']
    );
    expect(deliveryLabels(buatAnnouncement({ deliver_in_app: true, deliver_email: true }))).toEqual([
      'Notifikasi aplikasi',
      'Email',
    ]);
    expect(
      deliveryLabels(buatAnnouncement({ deliver_in_app: false, deliver_email: false }))
    ).toEqual([]);
  });
});

describe('bodyExcerpt', () => {
  it('membuang markup', () => {
    expect(bodyExcerpt('<p>halo <strong>dunia</strong></p>')).toBe('halo dunia');
  });

  it('tidak membocorkan isi script ke kutipan', () => {
    expect(bodyExcerpt('<script>alert(1)</script>teks asli')).toBe('teks asli');
  });

  it('memotong dengan elipsis', () => {
    const panjang = 'a'.repeat(200);
    const hasil = bodyExcerpt(panjang, 120);
    expect(hasil.endsWith('…')).toBe(true);
    expect(hasil.length).toBe(121);
  });

  it('mengubah &nbsp; menjadi spasi biasa', () => {
    expect(bodyExcerpt('<p>a&nbsp;b</p>')).toBe('a b');
  });

  it('menangani isi kosong', () => {
    expect(bodyExcerpt('')).toBe('');
    expect(bodyExcerpt('<p></p>')).toBe('');
  });
});
