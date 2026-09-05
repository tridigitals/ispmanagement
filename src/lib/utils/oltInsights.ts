//! Pure helpers for OLT pages — status, signal bands, friendly errors.
//! Murni (no DOM, no API) so they can be unit-tested without rendering.

import type { StatusTone } from '$lib/components/ds/tokens';

/** Driver registry server-side (drivers/mod.rs create_driver). A type listed
 *  in the UI but absent here silently breaks monitoring after save. */
export const OLT_DRIVER_TYPES = ['hioso_ha7302cst', 'mikrotik_ros'] as const;

export const OLT_TYPE_LABELS: Record<string, string> = {
  hioso_ha7302cst: 'HIOSO HA-7302CST (EPON)',
  vsol_epon: 'VSOL (EPON)',
  mikrotik_ros: 'MikroTik RouterOS (API)',
};

export function oltTypeLabel(t: string | null | undefined): string {
  if (!t) return '—';
  return OLT_TYPE_LABELS[t] ?? t;
}

export function hasOltDriver(t: string | null | undefined): boolean {
  return !!t && (OLT_DRIVER_TYPES as readonly string[]).includes(t);
}

/** ONU RX power bands (dBm), same thresholds the legacy detail page used. */
export function signalBand(dbm: number | null | undefined): 'good' | 'fair' | 'weak' | 'very_weak' | 'unknown' {
  if (dbm == null || !Number.isFinite(dbm)) return 'unknown';
  if (dbm > -20) return 'good';
  if (dbm >= -24) return 'fair';
  if (dbm >= -27) return 'weak';
  return 'very_weak';
}

export function signalLabel(dbm: number | null | undefined): string {
  switch (signalBand(dbm)) {
    case 'good': return 'Baik';
    case 'fair': return 'Cukup';
    case 'weak': return 'Lemah';
    case 'very_weak': return 'Sangat lemah';
    default: return '—';
  }
}

export function signalColor(dbm: number | null | undefined): string {
  switch (signalBand(dbm)) {
    case 'good': return 'var(--ds-positive, #16a34a)';
    case 'fair': return 'var(--ds-warning, #ca8a04)';
    case 'weak': return '#ea580c';
    case 'very_weak': return '#dc2626';
    default: return 'var(--ds-ink-400, #6f6f78)';
  }
}

/** Parse "−21.5 dBm" / "-21.5" / number → number|null. ONU rx/tx arrive as strings. */
export function parseDbm(v: unknown): number | null {
  if (typeof v === 'number') return Number.isFinite(v) ? v : null;
  if (typeof v !== 'string') return null;
  const n = parseFloat(v.replace(/−/g, '-'));
  return Number.isFinite(n) ? n : null;
}

export function onuStatusTone(status: string | null | undefined): StatusTone {
  const s = (status ?? '').toLowerCase().replace(/[\s_]+/g, '-');
  if (s === 'online') return 'positive';
  if (s === 'offline') return 'negative';
  if (s === 'los' || s === 'dying-gasp') return 'warning';
  return 'neutral';
}

export function formatUptime(seconds?: number | null): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return '—';
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}h ${h}j ${m}m`;
  if (h > 0) return `${h}j ${m}m`;
  return `${m}m`;
}

export function formatBytes(bytes?: number | null): string {
  if (bytes == null || !Number.isFinite(bytes)) return '—';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/** Client-side validation for the OLT form. Returns label→message; empty = ok. */
export function validateOltDraft(d: {
  name: string;
  host: string;
  port: number | null;
  username: string;
  password: string;
  oltType: string;
  latitude: number | null;
  longitude: number | null;
  isNew: boolean;
}): Record<string, string> {
  const errs: Record<string, string> = {};
  if (!d.name.trim()) errs.name = 'Nama wajib diisi.';
  if (!d.host.trim()) errs.host = 'Host wajib diisi.';
  else if (!/^[a-zA-Z0-9._:\-\[\]]+$/.test(d.host.trim()))
    errs.host = 'Host harus IP atau hostname yang valid.';
  if (d.port == null || !Number.isInteger(d.port) || d.port < 1 || d.port > 65535)
    errs.port = 'Port harus 1–65535.';
  if (!d.username.trim()) errs.username = 'Username wajib diisi.';
  if (d.isNew && !d.password.trim()) errs.password = 'Password wajib untuk OLT baru.';
  if (!d.oltType) errs.oltType = 'Pilih tipe OLT.';
  const hasLat = d.latitude != null;
  const hasLng = d.longitude != null;
  if (hasLat !== hasLng) errs.location = 'Latitude dan longitude harus diisi berpasangan.';
  else if (hasLat) {
    if (Math.abs(d.latitude as number) > 90) errs.location = 'Latitude harus antara −90 dan 90.';
    else if (Math.abs(d.longitude as number) > 180) errs.location = 'Longitude harus antara −180 dan 180.';
  }
  return errs;
}

/** Translate known server errors into human Indonesian. */
export function friendlyOltError(raw: string): string {
  const m = (raw ?? '').toString();
  if (/Unsupported OLT type/i.test(m))
    return 'Tipe OLT ini belum punya driver di server — monitoring tidak akan berjalan. Pilih tipe yang didukung.';
  if (/violates foreign key constraint "fk_olts_uplink_router"/i.test(m))
    return 'Router uplink tidak ditemukan di daftar router aktif.';
  if (/Connection failed/i.test(m)) return 'Gagal terhubung ke perangkat: ' + m.replace(/^.*Connection failed:\s*/i, '');
  if (/password authentication|database error/i.test(m)) return 'Terjadi kesalahan server. Coba lagi.';
  return m;
}
