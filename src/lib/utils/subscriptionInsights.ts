/**
 * Helper murni untuk halaman Langganan (plan platform) v2.
 *
 * Menggantikan daftar fitur HARDCODE per slug di versi lama
 * (getPlanFeatures: 'free' | 'pro' | 'enterprise' — copy marketing,
 * bukan entitlement nyata dari DB).
 */

export interface PlanFeatureRow {
  feature_id: string;
  code: string;
  name: string;
  value_type: string; // boolean | number | unlimited | text
  value: string;
  category: string;
}

/** Nilai fitur -> teks manusiawi Indonesia. 'unlimited' dan boolean
 *  disimpan sebagai string di DB. */
export function featureValueLabel(f: Pick<PlanFeatureRow, 'value_type' | 'value'>): string {
  const v = (f.value ?? '').trim();
  if (!v) return '—';
  if (v.toLowerCase() === 'unlimited') return 'Tanpa batas';
  if (f.value_type === 'boolean') return v === 'true' ? 'Ya' : 'Tidak';
  if (f.value_type === 'number') {
    const n = Number(v);
    if (Number.isFinite(n)) return n.toLocaleString('id-ID');
  }
  return v;
}

/** true bila fitur berarti "aktif/ada" untuk badge centang. */
export function featureIsOn(f: Pick<PlanFeatureRow, 'value_type' | 'value'>): boolean {
  const v = (f.value ?? '').trim().toLowerCase();
  if (!v) return false;
  if (v === 'unlimited') return true;
  if (f.value_type === 'boolean') return v === 'true';
  if (f.value_type === 'number') return Number(v) > 0;
  return true; // text non-kosong dianggap aktif
}

/** Kelompokkan fitur per kategori, urutan stabil (nama kategori asc,
 *  lalu urutan asal dari server yang sudah sort_order). */
export function groupFeaturesByCategory(
  features: PlanFeatureRow[],
): { category: string; items: PlanFeatureRow[] }[] {
  const map = new Map<string, PlanFeatureRow[]>();
  for (const f of features) {
    const cat = f.category || 'Lainnya';
    const arr = map.get(cat);
    if (arr) arr.push(f);
    else map.set(cat, [f]);
  }
  return Array.from(map.entries())
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([category, items]) => ({ category, items }));
}

/** Persen pemakaian terhadap limit; limit null = unlimited -> 0. */
export function usagePercent(used: number, limit: number | null | undefined): number {
  if (!limit || limit <= 0) return 0;
  return Math.min(100, Math.round((used / limit) * 100));
}

/** Tone progress bar dari persen. */
export function usageTone(pct: number): 'positive' | 'warning' | 'negative' {
  if (pct >= 100) return 'negative';
  if (pct > 80) return 'warning';
  return 'positive';
}

/** Byte -> satuan manusiawi (basis 1024). */
export function formatBytesIEC(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  const val = bytes / Math.pow(1024, i);
  return `${parseFloat(val.toFixed(2))} ${units[i]}`;
}

/** Pesan error backend plan/langganan -> Indonesia ramah. */
export function friendlyPlanError(raw: string | null | undefined): string {
  const msg = (raw ?? '').trim();
  if (!msg) return 'Terjadi kesalahan yang tidak diketahui.';
  const lower = msg.toLowerCase();
  if (lower.includes('still used by')) {
    const tenants = msg.split(':').slice(1).join(':').trim();
    return `Paket masih dipakai tenant: ${tenants || 'lihat detail'}. Pindahkan tenant dulu.`;
  }
  if (lower.includes('already exists')) return 'Slug paket sudah dipakai paket lain.';
  if (lower.includes('must be')) return `Data tidak valid: ${msg}`;
  if (lower.includes('free plans do not require')) return 'Paket gratis tidak memerlukan invoice.';
  if (lower.includes('plan not found')) return 'Paket tidak ditemukan — mungkin sudah dihapus.';
  if (lower.includes('database error') || lower.includes('internal server error')) {
    return 'Terjadi kesalahan di server. Coba lagi sebentar.';
  }
  return msg;
}
