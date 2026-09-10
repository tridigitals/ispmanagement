/**
 * Helper murni rekonsiliasi lifecycle v2 (gelombang 24b).
 *
 * Label isu/aksi/rekomendasi + format periode dulu inline `$t()` di
 * halaman legacy — kini pemetaan murni + tes.
 */
export type LifecycleIssueType = 'missing_bootstrap_invoice' | 'invalid_active_lifecycle';

export function lifecycleIssueLabel(issueType: string, tt?: (k: string) => string): string {
  if (issueType === 'missing_bootstrap_invoice') return tt?.('admin.lifecycle_recon.it_missing') ?? 'Belum ada invoice awal';
  if (issueType === 'invalid_active_lifecycle') return tt?.('admin.lifecycle_recon.it_invalid') ?? 'Lifecycle aktif tidak valid';
  return issueType;
}

export function lifecycleActionLabel(action: string, tt?: (k: string) => string): string {
  if (action === 'bootstrap_invoice') return tt?.('admin.lifecycle_recon.ac_boot') ?? 'Buat invoice awal';
  if (action === 'review_lifecycle_data') return tt?.('admin.lifecycle_recon.ac_review') ?? 'Tinjau data lifecycle';
  if (action === 'suspend_invalid_active_lifecycle') return tt?.('admin.lifecycle_recon.ac_suspend') ?? 'Suspend layanan';
  return action;
}

export function lifecycleServiceLabel(packageName?: string | null, locationLabel?: string | null): string {
  return `${packageName || 'Paket'} • ${locationLabel || 'Lokasi tak dikenal'}`;
}

export function lifecyclePeriod(startsAt?: string | null, endsAt?: string | null): string {
  const s = startsAt ? startsAt.slice(0, 10) : '—';
  const e = endsAt ? endsAt.slice(0, 10) : '—';
  return `${s} → ${e}`;
}
