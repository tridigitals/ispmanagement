/**
 * Turunan tampilan untuk halaman pengumuman admin.
 *
 * Dipisahkan dari komponen supaya bisa diuji tanpa merender Svelte, dan supaya
 * satu definisi status dipakai bersama alih-alih dihitung ulang di tiap layar.
 *
 * Temuan yang dikunci berkas ini (dibuktikan di data produksi tenant
 * ISP Management, 2026-09-04):
 *
 * 1. STATUS DIHITUNG DUA KALI DENGAN DUA RUMUS.
 *    Server memfilter status di SQL (`http/announcements.rs:526-555`), klien
 *    menghitung ulang lewat `statusOf()` lokal. Keduanya kebetulan sepakat
 *    karena `ends_at > starts_at` divalidasi server, tapi tidak ada yang
 *    menjamin itu bertahan. `announcementStatus` di sini adalah salinan
 *    sengaja dari semantik server, lengkap dengan alasannya.
 *
 * 2. PILIHAN AUDIENS MENJANGKAU HAMPIR NOL ORANG, DAN LAYAR TIDAK BILANG.
 *    Pengiriman in-app menyasar akun portal, bukan baris pelanggan. Di tenant
 *    ini 548 pelanggan hanya punya 2 akun portal, jadi:
 *      admins → 1 orang, customers → 2, active_subscribers → 0,
 *      suspended_subscribers → 1, all → 6.
 *    Dua pilihan menjangkau nol orang dan tampak sama saja dengan pilihan
 *    lain di dropdown.
 *
 * 3. SCOPE GLOBAL MENGABAIKAN AUDIENS SEPENUHNYA. (DI PERBAIKI GELOMBANG INI)
 *    Ketiga salinan pengirim (`http/announcements.rs:658`,
 *    `services/announcement_service.rs:322`, `commands/announcements.rs:63`)
 *    menjalankan `SELECT id FROM users WHERE is_active = true` untuk
 *    pengumuman global — nilai `audience` tidak pernah dibaca. Memilih
 *    "hanya admin" pada pengumuman global mengirim ke seluruh 18 user aktif.
 *    Sekarang keempat cabang menunjuk `global_recipient_ids()` yang menghormati
 *    audiens.
 *
 * 4. MENYUNTING PENGUMUMAN TERKIRIM TIDAK PERNAH MENGIRIM ULANG.
 *    `UPDATE announcements` menulis 13 kolom dan `notified_at` bukan salah
 *    satunya, sementara penjadwal hanya memilih baris `notified_at IS NULL`.
 *    Diuji langsung di DB (BEGIN/ROLLBACK): menggeser `starts_at` pengumuman
 *    yang sudah terkirim ke masa depan lalu kembali ke masa kini menghasilkan
 *    0 baris terpilih. Seluruh 16 pengumuman di tenant ini sudah `notified_at`,
 *    jadi setiap penyuntingan jadwal sejak saat itu tak berefek apa pun.
 *
 * 5. TANGGAL TAK VALID DIAM-DIAM MENJADI "TERBITKAN SEKARANG".
 *    `toIsoOrNull` lama mengembalikan null untuk tanggal yang tak bisa
 *    di-parse, dan server memaknai null sebagai `now()`. Salah ketik jadwal
 *    berarti pengumuman langsung tersiar.
 */
import type { Announcement } from '$lib/api/types';

export type AnnouncementStatus = 'active' | 'scheduled' | 'expired';

/** Jangkauan satu pilihan audiens: angka + apakah perlu diperingatkan. */
export interface AudienceReach {
  value: string;
  label: string;
  /** Jumlah akun yang benar-benar menerima, null bila belum dihitung. */
  recipients: number | null;
  /** Peringatan yang wajib tampil di layar, null bila tidak ada. */
  warning: string | null;
}

export interface DraftInput {
  title: string;
  body: string;
  startsAt: string;
  endsAt: string;
  deliverInApp: boolean;
  deliverEmail: boolean;
  scope: 'tenant' | 'global';
}

export interface DraftIssue {
  field: 'title' | 'body' | 'startsAt' | 'endsAt' | 'delivery' | 'audience';
  message: string;
}

/**
 * Status pengumuman menurut semantik SERVER.
 *
 * Urutan pemeriksaan penting dan sengaja mengikuti SQL:
 * kedaluwarsa butuh `ends_at` terisi DAN sudah lewat; terjadwal berarti
 * `starts_at` masih di depan; sisanya aktif. Jangan disederhanakan menjadi
 * perbandingan rentang, karena `ends_at` null berarti "tanpa akhir", bukan
 * "berakhir di epoch 0".
 */
export function announcementStatus(a: Announcement, now: number = Date.now()): AnnouncementStatus {
  const mulai = new Date(a.starts_at).getTime();
  const akhir = a.ends_at ? new Date(a.ends_at).getTime() : null;

  if (akhir !== null && akhir <= now) return 'expired';
  if (Number.isFinite(mulai) && mulai > now) return 'scheduled';
  return 'active';
}

/** Rekap jumlah per status, untuk kartu ringkasan. */
export function statusCounts(
  rows: Announcement[],
  now: number = Date.now()
): Record<AnnouncementStatus, number> {
  const hasil: Record<AnnouncementStatus, number> = { active: 0, scheduled: 0, expired: 0 };
  for (const r of rows) hasil[announcementStatus(r, now)] += 1;
  return hasil;
}

/**
 * Apakah pengumuman ini sudah pernah dikirim.
 *
 * Penting ditampilkan karena menentukan apakah penyuntingan masih berpengaruh
 * pada pengiriman: begitu `notified_at` terisi, penjadwal tidak akan pernah
 * menyentuh baris itu lagi.
 */
export function alreadyDelivered(a: Announcement): boolean {
  return Boolean(a.notified_at);
}

/**
 * Peringatan yang harus tampil saat pengguna membuka penyuntingan.
 *
 * Mengembalikan null bila pengumuman belum terkirim (penyuntingan aman).
 */
export function editDeliveryWarning(a: Announcement): string | null {
  if (!alreadyDelivered(a)) return null;
  return 'Pengumuman ini sudah dikirim. Mengubah isi memperbarui halaman yang dibuka penerima, tapi tidak mengirim notifikasi baru dan tidak menjadwalkan ulang pengiriman.';
}

/** Label kanal yang aktif. Pengumuman tanpa kanal tidak akan pernah dikirim. */
export function deliveryLabels(a: Announcement): string[] {
  const label: string[] = [];
  if (a.deliver_in_app) label.push('Notifikasi aplikasi');
  if (a.deliver_email) label.push('Email');
  return label;
}

/**
 * Susun pilihan audiens beserta jangkauan sebenarnya.
 *
 * `counts` diisi pemanggil dari data hidup. Pilihan yang menjangkau nol akun
 * tetap ditampilkan — menyembunyikannya akan menutupi masalahnya — tapi diberi
 * peringatan eksplisit supaya admin tidak menyangka pesannya terkirim.
 *
 * Dulu fungsi ini menerima `scope` dan memberi peringatan khusus untuk global
 * ("audiens diabaikan"). Setelah `global_recipient_ids` di Rust memperbaiki
 * cabang global, scope tidak lagi mengubah hasil — parameternya dibuang supaya
 * tidak meninggalkan asumsi usang.
 */
export function audienceOptions(
  counts: Partial<Record<string, number>> = {}
): AudienceReach[] {
  const dasar: Array<{ value: string; label: string }> = [
    { value: 'all', label: 'Semua (staf + pelanggan berakun)' },
    { value: 'admins', label: 'Admin tenant' },
    { value: 'customers', label: 'Pelanggan berakun portal' },
    { value: 'active_subscribers', label: 'Pelanggan berlangganan aktif' },
    { value: 'suspended_subscribers', label: 'Pelanggan tersuspensi' },
  ];

  return dasar.map(({ value, label }) => {
    const recipients = counts[value] ?? null;

    if (recipients === 0) {
      return {
        value,
        label,
        recipients,
        warning: 'Tidak ada akun yang cocok. Pengumuman tidak akan sampai ke siapa pun.',
      };
    }

    return { value, label, recipients, warning: null };
  });
}

/**
 * Ringkas kesenjangan antara jumlah pelanggan dan pelanggan yang bisa dihubungi.
 *
 * Null bila tidak ada kesenjangan, supaya layar tidak memasang peringatan
 * kosong pada tenant yang sudah beres.
 */
export function portalCoverageGap(totalCustomers: number, withPortalAccount: number): string | null {
  if (totalCustomers <= 0) return null;
  if (withPortalAccount >= totalCustomers) return null;
  const persen = Math.round((withPortalAccount / totalCustomers) * 100);
  return `${withPortalAccount} dari ${totalCustomers} pelanggan (${persen}%) punya akun portal. Notifikasi aplikasi hanya sampai ke akun portal, jadi sisanya tidak menerima pengumuman lewat kanal ini.`;
}

/**
 * Validasi draf di klien, meniru aturan server supaya galat muncul sebelum
 * berkas cover diunggah.
 *
 * Alur lama mengunggah cover LEBIH DULU (`+page.svelte:211-213`) lalu baru
 * memanggil `createAdmin`; kalau server menolak karena `ends_at <= starts_at`,
 * berkas yang sudah terunggah menjadi sampah tanpa pemilik.
 */
export function validateDraft(draft: DraftInput): DraftIssue[] {
  const masalah: DraftIssue[] = [];

  if (!draft.title.trim()) {
    masalah.push({ field: 'title', message: 'Judul wajib diisi.' });
  }
  if (!draft.body.trim() || !stripToText(draft.body)) {
    masalah.push({ field: 'body', message: 'Isi pengumuman wajib diisi.' });
  }
  if (!draft.deliverInApp && !draft.deliverEmail) {
    masalah.push({ field: 'delivery', message: 'Pilih minimal satu kanal pengiriman.' });
  }

  const mulai = parseWaktu(draft.startsAt);
  const akhir = parseWaktu(draft.endsAt);

  if (draft.startsAt.trim() && mulai === null) {
    masalah.push({
      field: 'startsAt',
      message: 'Tanggal mulai tidak bisa dibaca. Kosongkan untuk terbit sekarang.',
    });
  }
  if (draft.endsAt.trim() && akhir === null) {
    masalah.push({ field: 'endsAt', message: 'Tanggal berakhir tidak bisa dibaca.' });
  }
  if (mulai !== null && akhir !== null && akhir <= mulai) {
    masalah.push({
      field: 'endsAt',
      message: 'Tanggal berakhir harus setelah tanggal mulai.',
    });
  }

  return masalah;
}

/**
 * Parse nilai `datetime-local` menjadi epoch ms, atau null bila tak valid.
 *
 * Dipisah supaya kegagalan parse bisa dibedakan dari "sengaja dikosongkan" —
 * pembeda yang hilang di `toIsoOrNull` lama dan membuat salah ketik berubah
 * menjadi "terbitkan sekarang".
 */
export function parseWaktu(nilai: string): number | null {
  const s = (nilai || '').trim();
  if (!s) return null;
  const t = new Date(s).getTime();
  return Number.isNaN(t) ? null : t;
}

/** Ubah nilai form menjadi ISO untuk DTO, hanya bila valid. */
export function toIso(nilai: string): string | null {
  const t = parseWaktu(nilai);
  return t === null ? null : new Date(t).toISOString();
}

/** Ambil teks dari HTML tanpa bergantung DOM; cukup untuk cek "kosong". */
function stripToText(html: string): string {
  return html
    .replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1>/gi, '')
    .replace(/<[^>]*>/g, '')
    .replace(/&nbsp;/gi, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

/** Warna badge menurut tingkat kepentingan. */
export function severityTone(severity: string): 'neutral' | 'positive' | 'warning' | 'negative' {
  switch (severity) {
    case 'success':
      return 'positive';
    case 'warning':
      return 'warning';
    case 'error':
      return 'negative';
    default:
      return 'neutral';
  }
}

/** Warna badge menurut status. */
export function statusTone(status: AnnouncementStatus): 'neutral' | 'positive' | 'warning' {
  if (status === 'active') return 'positive';
  if (status === 'scheduled') return 'warning';
  return 'neutral';
}

export function statusLabel(status: AnnouncementStatus): string {
  if (status === 'active') return 'Tayang';
  if (status === 'scheduled') return 'Terjadwal';
  return 'Kedaluwarsa';
}

export function scopeLabel(a: Announcement): string {
  return a.tenant_id === null ? 'Global' : 'Tenant';
}

/** Kutipan isi untuk daftar, sudah bebas markup. */
export function bodyExcerpt(body: string, panjang = 120): string {
  const teks = stripToText(body);
  return teks.length > panjang ? `${teks.slice(0, panjang).trimEnd()}…` : teks;
}
