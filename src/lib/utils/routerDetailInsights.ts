/**
 * Helper murni halaman detail router v2 (gelombang 22).
 *
 * Logika format & agregasi yang dulu hidup di komponen 1.752 baris
 * (+ 493 dialog) dipindah ke sini supaya teruji:
 * - formatBps/formatBytes/formatUptime/pctUsed: pemformatan nilai
 *   live MikroTik (bps, byte, uptime, persen).
 * - interfaceRow/interfaceStatus: ubah InterfaceSnap -> baris tabel
 *   + status berurutan disabled > running > down.
 * - filterInterfaceRows: filter all/running/down/disabled.
 * - snapshotHealthStats: ringkasan kesehatan snapshot utk tile.
 * - friendlyRouterError: rapikan pesan error (404 router, offline,
 *   timeouts) jadi kalimat yang menjelaskan.
 */
export type InterfaceStatus = 'running' | 'down' | 'disabled';

export interface RouterHealthStats {
  online: boolean;
  cpu: number | null;
  memPct: number | null;
  diskPct: number | null;
  uptime: string;
}

const NA = '—';

export function formatBps(bps?: number | null): string {
  if (bps == null) return NA;
  const abs = Math.abs(bps);
  const units = ['bps', 'Kbps', 'Mbps', 'Gbps'];
  let u = 0;
  let v = abs;
  while (v >= 1000 && u < units.length - 1) {
    v /= 1000;
    u++;
  }
  const s = `${v >= 10 || u === 0 ? v.toFixed(0) : v.toFixed(1)} ${units[u]}`;
  return bps < 0 ? `-${s}` : s;
}

export function formatBytes(n?: number | null): string {
  if (n == null) return NA;
  const abs = Math.abs(n);
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let u = 0;
  let v = abs;
  while (v >= 1024 && u < units.length - 1) {
    v /= 1024;
    u++;
  }
  const s = `${v >= 10 || u === 0 ? v.toFixed(0) : v.toFixed(1)} ${units[u]}`;
  return n < 0 ? `-${s}` : s;
}

export function formatUptime(secs?: number | null): string {
  if (secs == null) return NA;
  const s = Math.max(0, Math.floor(secs));
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export function pctUsed(total?: number | null, free?: number | null): number | null {
  if (!total || total <= 0 || free == null) return null;
  const used = total - free;
  return Math.max(0, Math.min(100, Math.round((used / total) * 100)));
}

export interface InterfaceLike {
  name: string;
  interface_type?: string | null;
  running?: boolean | null;
  disabled?: boolean | null;
  mtu?: number | null;
  mac_address?: string | null;
  rx_byte?: number | null;
  tx_byte?: number | null;
  rx_packet?: number | null;
  tx_packet?: number | null;
  link_downs?: number | null;
}

export interface InterfaceRow extends InterfaceLike {
  status: InterfaceStatus;
}

export function interfaceStatus(it: InterfaceLike): InterfaceStatus {
  if (it.disabled) return 'disabled';
  if (it.running) return 'running';
  return 'down';
}

export function interfaceRows(list: InterfaceLike[] | undefined | null): InterfaceRow[] {
  return (list || []).map((it) => ({ ...it, status: interfaceStatus(it) }));
}

export function filterInterfaceRows(
  rows: InterfaceRow[],
  filter: 'all' | 'running' | 'down' | 'disabled',
): InterfaceRow[] {
  if (filter === 'all') return rows;
  return rows.filter((r) => r.status === filter);
}

export function snapshotHealthStats(opts: {
  isOnline: boolean;
  cpuLoad?: number | null;
  totalMemoryBytes?: number | null;
  freeMemoryBytes?: number | null;
  totalHddBytes?: number | null;
  freeHddBytes?: number | null;
  uptimeSeconds?: number | null;
}): RouterHealthStats {
  return {
    online: opts.isOnline,
    cpu: opts.cpuLoad == null ? null : Math.max(0, Math.min(100, opts.cpuLoad)),
    memPct: pctUsed(opts.totalMemoryBytes, opts.freeMemoryBytes),
    diskPct: pctUsed(opts.totalHddBytes, opts.freeHddBytes),
    uptime: formatUptime(opts.uptimeSeconds),
  };
}

export function friendlyRouterError(msg: string | null | undefined): string {
  const m = msg || '';
  if (!m) return 'Gagal memuat data router.';
  if (/router not found|not found/i.test(m)) return 'Router tidak ditemukan atau sudah dihapus.';
  if (/timed? ?out|timeout|timed out/i.test(m)) return 'Router tidak merespons — koneksi timeout.';
  if (/offline|no route|unreachable|connection refused|econnrefused/i.test(m))
    return 'Router sedang offline atau tidak terjangkau.';
  return m;
}
