export type PppoeAccountSource = 'router' | 'managed_radius';

export type PppoeSyncDisplayInput = {
  account_source: PppoeAccountSource;
  router_present: boolean;
  is_provisioned: boolean;
  last_sync_at: string | null;
  provisioned_at: string | null;
  last_error: string | null;
  provisioning_error: string | null;
};

export type PppoeSyncDisplay = {
  label: string;
  tone: 'ok' | 'warn';
  syncedAt: string | null;
  error: string | null;
};

export function getPppoeProvisioningTargetFallback(source: PppoeAccountSource): string {
  return source === 'managed_radius' ? 'RADIUS' : 'router';
}

export function getPppoeApplyActionFallback(source: PppoeAccountSource): string {
  return `Apply to ${getPppoeProvisioningTargetFallback(source)}`;
}

export function getPppoeCreateActionFallback(source: PppoeAccountSource): string {
  return `Create & apply to ${getPppoeProvisioningTargetFallback(source)}`;
}

export function getPppoeCreatedAndAppliedToastFallback(source: PppoeAccountSource): string {
  return `PPPoE account created and applied to ${getPppoeProvisioningTargetFallback(source)}`;
}

export function getPppoeSyncDisplay(row: PppoeSyncDisplayInput): PppoeSyncDisplay {
  if (row.account_source === 'managed_radius') {
    return {
      label: row.is_provisioned ? 'Provisioned' : 'Not provisioned',
      tone: row.is_provisioned ? 'ok' : 'warn',
      syncedAt: row.provisioned_at,
      error: row.provisioning_error,
    };
  }

  return {
    label: row.router_present ? 'On router' : 'Missing',
    tone: row.router_present ? 'ok' : 'warn',
    syncedAt: row.last_sync_at,
    error: row.last_error,
  };
}
