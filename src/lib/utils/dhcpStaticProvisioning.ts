import type { DhcpStaticServicePublic } from '$lib/api/types';

export type DhcpStaticProvisioningStatus = 'draft' | 'apply_failed' | 'applied' | 'applied_queue';

type DhcpStaticProvisioningState = Pick<
  DhcpStaticServicePublic,
  'lease_present' | 'lease_last_error' | 'queue_mode' | 'queue_present' | 'queue_last_error'
>;

export function getDhcpStaticProvisioningStatus(
  service: DhcpStaticProvisioningState | null | undefined,
): DhcpStaticProvisioningStatus {
  if (!service) return 'draft';
  if (hasProvisioningError(service)) return 'apply_failed';
  if (!service.lease_present) return 'draft';
  if (service.queue_mode === 'simple_queue' && service.queue_present) return 'applied_queue';
  return 'applied';
}

export function isDhcpStaticProvisioningReady(
  service: DhcpStaticProvisioningState | null | undefined,
): boolean {
  return Boolean(service?.lease_present && !service.lease_last_error?.trim());
}

export function getDhcpStaticProvisioningError(
  service: DhcpStaticProvisioningState | null | undefined,
): string | null {
  if (!service) return null;
  return service.lease_last_error?.trim() || service.queue_last_error?.trim() || null;
}

function hasProvisioningError(service: DhcpStaticProvisioningState): boolean {
  return Boolean(service.lease_last_error?.trim() || service.queue_last_error?.trim());
}
