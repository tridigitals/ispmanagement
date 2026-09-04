/**
 * Status router yang jujur.
 *
 * KENAPA INI ADA.
 *
 * Halaman lama (`(app)/admin/network/routers/+page.svelte`) menurunkan status
 * dari SATU kolom saja:
 *
 *   baris  94: online = routers.filter(r => r.is_online).length
 *   baris 317: statusLabel() -> is_online ? 'Online' : 'Offline'
 *   baris 416: badge class:online={item.is_online}
 *
 * `enabled` ada di tipe barisnya (baris 25) tapi tidak pernah dipakai di tabel,
 * hanya di form. Masalahnya: poller backend memfilter
 * `WHERE enabled = true` (`mikrotik_service.rs:2349`), jadi begitu sebuah router
 * dinonaktifkan ia BERHENTI diperbarui dan `is_online` MEMBEKU pada nilai
 * terakhirnya — selamanya.
 *
 * Terbukti di data hidup 2026-09-04: router "Solikin" `enabled=false`,
 * `is_online=true`, `latency_ms=65`, `last_seen_at` 28 hari lalu. Layar
 * menampilkan badge hijau "Online" dan "65 ms" tanpa penanda apa pun bahwa
 * router itu dimatikan dan angkanya basi hampir sebulan.
 *
 * Ringkasan ikut salah: "Online 3 dari 3" padahal hanya 2 router di tenant itu
 * yang benar-benar dipoll.
 *
 * Aturan di sini: status TIDAK BOLEH ditentukan oleh satu boolean. `enabled`
 * dan umur `last_seen_at` ikut menentukan, dan setiap status membawa alasan
 * yang bisa dibaca pengguna.
 */

/** Interval poller backend, `MIKROTIK_POLL_INTERVAL_SECS` (default 300s). */
export const POLL_INTERVAL_MS = 300_000;

/**
 * Ambang data dianggap basi: 3x interval poll. Satu siklus terlewat bisa
 * karena jaringan; tiga siklus berarti ada yang salah.
 */
export const STALE_AFTER_MS = POLL_INTERVAL_MS * 3;

export type RouterState = 'online' | 'offline' | 'stale' | 'disabled' | 'maintenance';

export interface RouterLike {
  enabled?: boolean | null;
  is_online?: boolean | null;
  last_seen_at?: string | null;
  latency_ms?: number | null;
  maintenance_until?: string | null;
  maintenance_reason?: string | null;
  last_error?: string | null;
}

export interface RouterStatus {
  state: RouterState;
  label: string;
  /** Alasan yang bisa dibaca pengguna. Selalu ada, tidak pernah kosong. */
  reason: string;
  /** Umur data dalam ms, null kalau belum pernah terlihat. */
  ageMs: number | null;
  /**
   * Boleh menampilkan latensi/metrik? False untuk disabled & stale supaya
   * angka beku tidak tampil seolah pengukuran baru.
   */
  metricsTrustworthy: boolean;
}

const MENIT = 60_000;
const JAM = 3_600_000;
const HARI = 86_400_000;

/** "28 hari", "3 jam", "5 menit", "baru saja". */
export function humanAge(ms: number): string {
  if (ms < MENIT) return 'baru saja';
  if (ms < JAM) return `${Math.floor(ms / MENIT)} menit`;
  if (ms < HARI) return `${Math.floor(ms / JAM)} jam`;
  return `${Math.floor(ms / HARI)} hari`;
}

export function routerStatus(r: RouterLike, now: number = Date.now()): RouterStatus {
  const seenMs = r.last_seen_at ? new Date(r.last_seen_at).getTime() : NaN;
  const ageMs = Number.isFinite(seenMs) ? Math.max(0, now - seenMs) : null;
  const umur = ageMs == null ? 'belum pernah terhubung' : `${humanAge(ageMs)} lalu`;

  /* Dinonaktifkan menang atas segalanya: poller melewatkannya, jadi apa pun
     isi is_online/latency_ms adalah sisa masa lalu. */
  if (r.enabled === false) {
    return {
      state: 'disabled',
      label: 'Dinonaktifkan',
      reason: `Tidak dipantau. Data terakhir ${umur}.`,
      ageMs,
      metricsTrustworthy: false,
    };
  }

  const untilMs = r.maintenance_until ? new Date(r.maintenance_until).getTime() : NaN;
  if (Number.isFinite(untilMs) && untilMs > now) {
    const sisa = humanAge(untilMs - now);
    return {
      state: 'maintenance',
      label: 'Pemeliharaan',
      reason: r.maintenance_reason?.trim()
        ? `${r.maintenance_reason.trim()} — sisa ${sisa}.`
        : `Dijadwalkan selesai dalam ${sisa}.`,
      ageMs,
      metricsTrustworthy: Boolean(r.is_online),
    };
  }

  /* Aktif tapi tidak ada kabar berkali-kali siklus: jangan tampilkan hijau
     hanya karena kolom is_online belum diperbarui. */
  if (ageMs != null && ageMs > STALE_AFTER_MS) {
    return {
      state: 'stale',
      label: 'Data usang',
      reason: `Aktif, tapi tidak ada pembaruan sejak ${umur}. Poller berjalan tiap ${Math.round(
        POLL_INTERVAL_MS / 60000,
      )} menit.`,
      ageMs,
      metricsTrustworthy: false,
    };
  }

  if (r.is_online) {
    return {
      state: 'online',
      label: 'Online',
      reason: `Terakhir menjawab ${umur}.`,
      ageMs,
      metricsTrustworthy: true,
    };
  }

  return {
    state: 'offline',
    label: 'Offline',
    reason: r.last_error?.trim()
      ? `${r.last_error.trim()}`
      : ageMs == null
        ? 'Belum pernah berhasil terhubung.'
        : `Tidak menjawab. Terakhir terlihat ${umur}.`,
    ageMs,
    metricsTrustworthy: false,
  };
}

/**
 * Ringkasan yang menjumlah SEMUA router dan tidak menyembunyikan yang
 * dinonaktifkan. Versi lama menghitung `total = routers.length` lalu
 * `offline = total - online`, sehingga router dinonaktifkan ikut ke ember
 * "online" dan tidak ada ember untuk "tidak dipantau".
 */
export function summarize(rows: RouterLike[], now: number = Date.now()) {
  let online = 0;
  let offline = 0;
  let stale = 0;
  let disabled = 0;
  let maintenance = 0;

  for (const r of rows) {
    switch (routerStatus(r, now).state) {
      case 'online':
        online++;
        break;
      case 'offline':
        offline++;
        break;
      case 'stale':
        stale++;
        break;
      case 'disabled':
        disabled++;
        break;
      case 'maintenance':
        maintenance++;
        break;
    }
  }

  const monitored = rows.length - disabled;
  return { total: rows.length, monitored, online, offline, stale, disabled, maintenance };
}

/** Pemetaan ke tone Badge design system. */
export function statusTone(state: RouterState): 'positive' | 'negative' | 'warning' | 'neutral' {
  switch (state) {
    case 'online':
      return 'positive';
    case 'offline':
      return 'negative';
    case 'stale':
      return 'warning';
    case 'maintenance':
      return 'warning';
    case 'disabled':
      return 'neutral';
  }
}
