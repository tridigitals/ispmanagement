/**
 * Helper murni halaman detail pelanggan v2 (gelombang 21).
 *
 * Kolom `notes`/pesan error FK backend dan agregasi kesehatan pelanggan
 * dulu terjerat di komponen 3.865 baris. Fungsi di sini murni dan teruji;
 * halaman v2 tinggal memanggil.
 */

/** Label status langganan (konsisten dengan legacy + gelombang billing). */
export function subStatusLabel(status: string): string {
  const map: Record<string, string> = {
    active: 'Aktif',
    grace_active: 'Aktif sementara',
    pending_installation: 'Menunggu instalasi',
    installation_done_awaiting_payment: 'Menunggu pembayaran',
    suspended: 'Ditangguhkan',
    cancelled: 'Dibatalkan',
  };
  return map[status] || status;
}

export type SubStatusTone = 'positive' | 'warning' | 'negative' | 'neutral' | 'info';

export function subStatusTone(status: string): SubStatusTone {
  if (status === 'active' || status === 'grace_active') return 'positive';
  if (status === 'pending_installation' || status === 'installation_done_awaiting_payment') return 'warning';
  if (status === 'suspended') return 'negative';
  return 'neutral';
}

/**
 * Terjemahkan pesan guard FK backend (gelombang 21) jadi kalimat Indonesia
 * yang bisa dibaca admin, mis.
 * `cannot delete: still referenced by 3 subscriptions, 1 work orders`
 * -> `Tidak bisa dihapus — masih dipakai oleh 3 langganan, 1 work order.`
 */
export function friendlyCustomerError(raw: string | null | undefined): string {
  const msg = `${raw || ''}`.trim();
  if (!msg) return 'Terjadi kesalahan.';
  const ref = msg.match(/still referenced by (.+)$/i);
  if (ref) {
    const label: Record<string, string> = {
      subscriptions: 'langganan',
      'work orders': 'work order',
      'pppoe accounts': 'akun PPPoE',
      'dhcp services': 'layanan DHCP statis',
      locations: 'lokasi',
    };
    const parts = ref[1]
      .split(',')
      .map((p) => p.trim())
      .filter(Boolean)
      .map((p) => {
        const m = p.match(/^(\d+)\s+(.+)$/);
        if (!m) return p;
        return `${m[1]} ${label[m[2]] || m[2]}`;
      });
    return `Tidak bisa dihapus — masih dipakai oleh ${parts.join(', ')}.`;
  }
  if (/customer not found/i.test(msg)) return 'Pelanggan tidak ditemukan.';
  if (/subscription not found/i.test(msg)) return 'Langganan tidak ditemukan.';
  if (/location not found/i.test(msg)) return 'Lokasi tidak ditemukan.';
  if (/permission/i.test(msg)) return 'Anda tidak punya izin untuk aksi ini.';
  return msg;
}

export type CustomerHealthInput = {
  is_active: boolean;
  subscriptions: Array<{ status: string }>;
  pendingInstallations: number;
  suspendedCount?: number;
};

export type CustomerHealthChip = {
  key: string;
  label: string;
  tone: 'positive' | 'warning' | 'negative' | 'neutral';
};

/** Ringkasan kesehatan: chip untuk header detail pelanggan. */
export function customerHealthChips(input: CustomerHealthInput): CustomerHealthChip[] {
  const chips: CustomerHealthChip[] = [];
  if (!input.is_active) {
    chips.push({ key: 'inactive', label: 'Nonaktif', tone: 'neutral' });
  }
  const active = input.subscriptions.filter((s) => s.status === 'active' || s.status === 'grace_active').length;
  const suspended = input.subscriptions.filter((s) => s.status === 'suspended').length;
  const pending = input.subscriptions.filter((s) => s.status === 'pending_installation').length;
  if (active > 0) chips.push({ key: 'active', label: `${active} langganan aktif`, tone: 'positive' });
  if (pending > 0) chips.push({ key: 'pending', label: `${pending} menunggu instalasi`, tone: 'warning' });
  if (suspended > 0) chips.push({ key: 'suspended', label: `${suspended} ditangguhkan`, tone: 'negative' });
  if (input.subscriptions.length === 0 && input.is_active) {
    chips.push({ key: 'none', label: 'Belum ada langganan', tone: 'neutral' });
  }
  return chips;
}

/** Alamat satu baris untuk tabel lokasi. */
export function formatLocationLine(loc: {
  label: string;
  address_line1?: string | null;
  city?: string | null;
}): string {
  const parts = [loc.address_line1, loc.city].map((p) => `${p || ''}`.trim()).filter(Boolean);
  return parts.length ? `${loc.label} — ${parts.join(', ')}` : loc.label;
}

/**
 * Invoice mana yang dianggap "tagihan pelanggan ini". Legacy menarik SEMUA
 * invoice paket se-tenant lalu cocokkan client-side: subscription id
 * tersandi di `external_id` berformat `pkgsub:<id>:<ts>`. Helper ini
 * memurnikan predikatnya (termasuk parsing-nya) supaya bisa diuji.
 */
export function subscriptionIdFromInvoice(inv: { external_id?: string | null }): string | null {
  const ext = inv.external_id || '';
  if (!ext.startsWith('pkgsub:')) return null;
  const raw = ext.slice('pkgsub:'.length);
  const idx = raw.indexOf(':');
  if (idx <= 0) return null;
  return raw.slice(0, idx);
}

export function invoicesForSubscriptions<T extends { external_id?: string | null }>(
  invoices: T[],
  subscriptionIds: Set<string>,
): T[] {
  return invoices.filter((i) => {
    const sid = subscriptionIdFromInvoice(i);
    return !!sid && subscriptionIds.has(sid);
  });
}
