/*
  Helper murni halaman audit log v2.

  Kenapa file ini ada (probe data produksi 2026-09-04, tabel 21.706 baris):

  1. AKTOR TANPA NAMA. 14.331 baris punya user_id NULL (aksi sistem:
     billing.collection_run 13.594, mikrotik_alert 443, status_online/offline,
     dll.) dan 164 baris login_failed menunjuk user yang sudah dihapus
     (join miss). UI lama menampilkan "—" polos untuk KEDUA kasus, jadi
     "sistem menjalankan X" tidak bisa dibedakan dari "user terhapus
     melakukan Y". Bedanya penting saat mengaudit insiden.

  2. DETAILS CAMPURAN. 20.667 baris JSON, 1.000 baris teks bebas
     ("Latency alert: 955ms on Xtrabit", "Created router 'Xtrabit'...").
     Perilaku lama: teks bebas tampil mentah — oke — tapi JSON tampil
     apa adanya tanpa ringkasan. Helper ini menormalkan keduanya ke satu
     bentuk {kind, summary} untuk tabel; modal detail menampilkan penuh.

  3. RESOURCE = KODE INGGRES. 15 nilai resource berbeda (billing, settings,
     auth, mikrotik_alert, ...) ditampilkan mentah. Label Indonesia membuat
     filter dropdown terbaca.

  Aturan yang sama dengan helper gelombang lain: TANPA DOM, TANPA api,
  TANPA store — fungsi murni yang bisa diuji tanpa Svelte.
*/

export interface AuditLogRow {
  id: string;
  user_id: string | null;
  tenant_id: string | null;
  action: string;
  resource: string;
  resource_id: string | null;
  resource_name?: string;
  details: string | null;
  ip_address: string | null;
  created_at: string;
  user_name?: string;
  user_email?: string;
  tenant_name?: string;
}

export type ActorKind = 'user' | 'system' | 'deleted' | 'anonymous';

export interface ActorInfo {
  kind: ActorKind;
  /** Teks pendek untuk sel tabel. */
  label: string;
  /** Penjelasan panjang untuk tooltip/modal. */
  detail: string;
}

/**
 * Klasifikasi aktor berdasar BUKTI join, bukan asumsi.
 * - ada nama/email  -> user aktif
 * - user_id NULL    -> sistem (pekerja latar, bukan manusia)
 * - user_id ada tapi nama tidak -> user sudah dihapus dari tabel users
 * - login gagal tanpa akun cocok -> anonim (penerjemahan oleh pemanggil
 *   lewat aksi; di sini tetap 'deleted' karena bentuk datanya sama)
 */
export function describeActor(log: AuditLogRow): ActorInfo {
  if (log.user_name || log.user_email) {
    const label = log.user_name && log.user_email
      ? `${log.user_name} — ${log.user_email}`
      : (log.user_name || log.user_email || '');
    return { kind: 'user', label, detail: label };
  }
  if (!log.user_id) {
    return {
      kind: 'system',
      label: 'Sistem',
      detail: 'Aksi otomatis oleh pekerja latar (bukan manusia)',
    };
  }
  return {
    kind: 'deleted',
    label: 'User terhapus',
    detail: `user_id ${log.user_id} tidak ada lagi di tabel users`,
  };
}

export interface DetailsSummary {
  kind: 'json' | 'text' | 'empty';
  /** Ringkasan satu baris untuk tabel. */
  summary: string;
  /** Nilai terurai bila JSON objek (untuk modal). */
  fields: Array<{ key: string; value: string }>;
}

function stringify(v: unknown): string {
  if (v === null || v === undefined) return '—';
  if (typeof v === 'object') return JSON.stringify(v);
  return String(v);
}

/**
 * details bisa JSON atau teks bebas (1.000 baris teks bebas di produksi).
 * JSON.parse yang gagal BUKAN error — itu bentuk data yang sah di sini.
 */
export function summarizeDetails(details: string | null): DetailsSummary {
  if (!details || !details.trim()) {
    return { kind: 'empty', summary: '—', fields: [] };
  }
  const trimmed = details.trim();
  if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
    try {
      const parsed: unknown = JSON.parse(trimmed);
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        const entries = Object.entries(parsed as Record<string, unknown>);
        return {
          kind: 'json',
          summary: entries.map(([k, v]) => `${k}=${stringify(v)}`).join(' · ') || '—',
          fields: entries.map(([key, value]) => ({ key, value: stringify(value) })),
        };
      }
      // Array atau scalar JSON valid — tampilkan sebagai teks.
      return { kind: 'text', summary: trimmed, fields: [] };
    } catch {
      // Maksudnya JSON tapi rusak (mis. terpotong) — tandai sebagai teks
      // supaya tidak pernah menampilkan "{}" kosong yang menyesatkan.
      return { kind: 'text', summary: trimmed, fields: [] };
    }
  }
  return { kind: 'text', summary: trimmed, fields: [] };
}

/** Label Indonesia untuk nilai resource yang ada di produksi. */
const RESOURCE_LABELS: Record<string, string> = {
  billing: 'Penagihan',
  settings: 'Pengaturan',
  auth: 'Autentikasi',
  mikrotik_alert: 'Peringatan router',
  mikrotik_router: 'Router',
  support_ticket: 'Tiket dukungan',
  customer_subscriptions: 'Langganan',
  installation_work_orders: 'Order instalasi',
  file_records: 'Berkas',
  pppoe: 'PPPoE',
  customers: 'Pelanggan',
  announcements: 'Pengumuman',
  invoice: 'Faktur',
  ftth_assets: 'Aset FTTH',
  customer_users: 'Akun portal',
  user: 'Pengguna',
  users: 'Pengguna',
  tenant: 'Tenant',
  tenants: 'Tenant',
  roles: 'Peran',
  customer_locations: 'Lokasi pelanggan',
};

export function resourceLabel(resource: string): string {
  return RESOURCE_LABELS[resource] || resource;
}

/**
 * Nada badge per aksi. login_failed/login_locked = merah; create/publish =
 * hijau; sisanya netral. Prefix bertitik (billing.collection_run) dipisah
 * supaya keluarga aksi satu modul bernada sama.
 */
export function actionTone(action: string): 'positive' | 'negative' | 'warning' | 'neutral' {
  const a = action.toLowerCase();
  if (/(failed|locked|blocked|revoked|delete|hapus)/.test(a)) return 'negative';
  if (/(warn|alert|suspend|offline)/.test(a)) return 'warning';
  if (/(^|\.|_)(create|register|publish|assign|online)/.test(a)) return 'positive';
  return 'neutral';
}

/**
 * Rentang tanggal untuk filter. date_to BARE (YYYY-MM-DD) harus menutupi
 * SEHARI PENUH — perilaku lama `<= 00:00:00` membuat hari terakhir selalu
 * kosong. Input datetime-local (ada jam) lewat apa adanya.
 */
export function toIsoRange(
  from: string,
  to: string,
  now: () => Date = () => new Date(),
): { date_from?: string; date_to?: string } {
  void now; // dipertahankan agar pemanggil bisa menyuntik jam bila perlu
  const out: { date_from?: string; date_to?: string } = {};
  if (from) {
    out.date_from = /^\d{4}-\d{2}-\d{2}$/.test(from)
      ? new Date(`${from}T00:00:00`).toISOString()
      : new Date(from).toISOString();
  }
  if (to) {
    out.date_to = /^\d{4}-\d{2}-\d{2}$/.test(to)
      ? new Date(`${to}T23:59:59.999`).toISOString()
      : new Date(to).toISOString();
  }
  return out;
}

/**
 * Validasi lokal sebelum request: dari tidak boleh setelah sampai.
 * Mengembalikan pesan error atau null bila sah.
 */
export function validateDateRange(from: string, to: string): string | null {
  if (!from || !to) return null;
  const a = new Date(from).getTime();
  const b = new Date(to).getTime();
  if (Number.isNaN(a) || Number.isNaN(b)) return 'Tanggal tidak dapat dibaca.';
  if (a > b) return 'Tanggal mulai tidak boleh setelah tanggal akhir.';
  return null;
}
