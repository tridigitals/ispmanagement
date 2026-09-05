import type { StatusTone } from '$lib/components/ds/tokens';

/**
 * Helper murni untuk halaman DHCP Static v2.
 *
 * Baris dhcp_static_services punya DUA dimensi sinkronisasi terpisah:
 * lease (pemetaan MAC->IP) dan queue (pembatas bandwidth). Keduanya bisa
 * berbeda kondisi di router — halaman lama hanya menampilkan pill
 * "Present/Missing" tanpa menjelaskan mana yang bermasalah.
 */

export interface DhcpSyncRow {
  lease_present: boolean;
  lease_last_error: string | null;
  lease_last_sync_at: string | null;
  queue_mode: string;
  queue_present: boolean;
  queue_last_error: string | null;
  queue_rate_limit: string | null;
  disabled: boolean;
}

/** Kondisi sinkronisasi gabungan untuk badge + filter cepat. */
export type DhcpSyncState = 'synced' | 'partial' | 'missing' | 'error' | 'disabled';

export function dhcpSyncState(row: DhcpSyncRow): DhcpSyncState {
  if (row.disabled) return 'disabled';
  const leaseBad = !row.lease_present || Boolean(row.lease_last_error);
  const wantsQueue = row.queue_mode !== 'none';
  const queueBad = wantsQueue && (!row.queue_present || Boolean(row.queue_last_error));
  if (leaseBad && queueBad) return 'error';
  if (leaseBad || queueBad) return row.lease_present ? 'partial' : 'missing';
  return 'synced';
}

export function dhcpSyncTone(state: DhcpSyncState): StatusTone {
  switch (state) {
    case 'synced':
      return 'positive';
    case 'partial':
      return 'warning';
    case 'missing':
      return 'negative';
    case 'error':
      return 'negative';
    case 'disabled':
      return 'neutral';
  }
}

export function dhcpSyncLabel(state: DhcpSyncState): string {
  switch (state) {
    case 'synced':
      return 'Sinkron';
    case 'partial':
      return 'Sebagian';
    case 'missing':
      return 'Belum di router';
    case 'error':
      return 'Bermasalah';
    case 'disabled':
      return 'Dinonaktifkan';
  }
}

/** Ringkasan manusiawi dua dimensi sync untuk tooltip/modal. */
export function dhcpSyncSummary(row: DhcpSyncRow): string {
  const parts: string[] = [];
  if (row.lease_present) {
    parts.push('Lease ada di router');
  } else if (row.lease_last_error) {
    parts.push(`Lease gagal: ${row.lease_last_error}`);
  } else {
    parts.push('Lease belum diterapkan ke router');
  }
  if (row.queue_mode === 'none') {
    parts.push('tanpa pembatas bandwidth');
  } else if (row.queue_present) {
    parts.push(`Queue aktif${row.queue_rate_limit ? ` (${row.queue_rate_limit})` : ''}`);
  } else if (row.queue_last_error) {
    parts.push(`Queue gagal: ${row.queue_last_error}`);
  } else {
    parts.push('Queue belum diterapkan');
  }
  return parts.join(' • ');
}

/** Pesan error backend -> bahasa pengguna. */
export function friendlyDhcpError(raw: string | null | undefined): string {
  const msg = (raw || '').trim();
  if (!msg) return 'Gagal memproses permintaan.';
  const lower = msg.toLowerCase();
  if (lower.includes('duplicate dhcp static')) {
    return 'Kombinasi langganan, MAC, atau IP ini sudah terdaftar sebagai layanan DHCP static.';
  }
  if (lower.includes('mac_address must use')) {
    return 'Format MAC address tidak valid. Contoh: AA:BB:CC:DD:EE:FF';
  }
  if (lower.includes('ip_address must contain')) {
    return 'IP address tidak valid. Gunakan IPv4/IPv6, contoh: 10.10.20.55';
  }
  if (lower.includes('not found')) {
    return 'Layanan tidak ditemukan — mungkin sudah dihapus tab lain.';
  }
  if (lower.includes('routeros trap')) {
    return 'Router menolak perintah. Periksa nama DHCP server dan status koneksi.';
  }
  if (lower.includes('queue failed') || lower.includes('but queue')) {
    return msg
      .replace('Lease applied, but queue failed:', 'Lease berhasil, tapi queue gagal:')
      .trim();
  }
  return msg;
}
