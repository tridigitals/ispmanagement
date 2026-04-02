export type PppoeAccountSource = 'router' | 'managed_radius';

export function getPppoeProvisioningTargetFallback(source: PppoeAccountSource): string {
  return source === 'managed_radius' ? 'RADIUS' : 'router';
}

export function getPppoeApplyActionFallback(source: PppoeAccountSource): string {
  return `Apply to ${getPppoeProvisioningTargetFallback(source)}`;
}
