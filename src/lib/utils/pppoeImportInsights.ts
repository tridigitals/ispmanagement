/**
 * Helper murni impor PPPoE v2 (gelombang 24c).
 *
 * Label/tone aksi impor, validasi pasangan pelanggan-lokasi, dan ringkasan
 * kandidat dulu inline di halaman legacy — kini murni + tes.
 */
export type PppoeCandidate = {
  action: 'new' | 'update' | 'same';
  username: string;
  profile_name?: string | null;
  remote_address?: string | null;
  disabled?: boolean;
  password_available?: boolean;
};

export function pppoeActionLabel(action: string): string {
  if (action === 'new') return 'Baru';
  if (action === 'update') return 'Perbarui';
  return 'Sama';
}

export function pppoeActionTone(action: string): 'positive' | 'warning' | 'neutral' {
  if (action === 'new') return 'positive';
  if (action === 'update') return 'warning';
  return 'neutral';
}

export function pppoeMappingError(customerId: string, locationId: string): string | null {
  if ((customerId && !locationId) || (!customerId && locationId)) {
    return 'Pilih pelanggan dan lokasi dua-duanya, atau kosongkan dua-duanya.';
  }
  return null;
}

export function pppoeDefaultSelection(rows: PppoeCandidate[]): string[] {
  return rows.filter((c) => c.action === 'new' || c.action === 'update').map((c) => c.username);
}

export function pppoeSummary(rows: PppoeCandidate[]): { total: number; fresh: number; updates: number; same: number } {
  return {
    total: rows.length,
    fresh: rows.filter((c) => c.action === 'new').length,
    updates: rows.filter((c) => c.action === 'update').length,
    same: rows.filter((c) => c.action === 'same').length,
  };
}
