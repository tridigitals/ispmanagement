export type PppoeAccountSource = 'router' | 'managed_radius';

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
