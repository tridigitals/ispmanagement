import { describe, expect, it } from 'vitest';

import {
  getDhcpStaticProvisioningError,
  getDhcpStaticProvisioningStatus,
  isDhcpStaticProvisioningReady,
} from './dhcpStaticProvisioning';

const base = {
  lease_present: false,
  lease_last_error: null,
  queue_mode: 'none',
  queue_present: false,
  queue_last_error: null,
};

describe('dhcpStaticProvisioning', () => {
  it('treats missing or unapplied service as draft', () => {
    expect(getDhcpStaticProvisioningStatus(null)).toBe('draft');
    expect(getDhcpStaticProvisioningStatus(base)).toBe('draft');
    expect(isDhcpStaticProvisioningReady(base)).toBe(false);
  });

  it('surfaces lease failures before draft state', () => {
    const service = { ...base, lease_last_error: 'router rejected lease' };

    expect(getDhcpStaticProvisioningStatus(service)).toBe('apply_failed');
    expect(getDhcpStaticProvisioningError(service)).toBe('router rejected lease');
    expect(isDhcpStaticProvisioningReady(service)).toBe(false);
  });

  it('marks a successfully applied lease ready', () => {
    const service = { ...base, lease_present: true };

    expect(getDhcpStaticProvisioningStatus(service)).toBe('applied');
    expect(getDhcpStaticProvisioningError(service)).toBeNull();
    expect(isDhcpStaticProvisioningReady(service)).toBe(true);
  });

  it('tracks queue sync without making queue mandatory for completion', () => {
    const service = {
      ...base,
      lease_present: true,
      queue_mode: 'simple_queue',
      queue_present: true,
    };
    const queueFailure = {
      ...service,
      queue_present: false,
      queue_last_error: 'queue rejected',
    };

    expect(getDhcpStaticProvisioningStatus(service)).toBe('applied_queue');
    expect(getDhcpStaticProvisioningStatus(queueFailure)).toBe('apply_failed');
    expect(getDhcpStaticProvisioningError(queueFailure)).toBe('queue rejected');
    expect(isDhcpStaticProvisioningReady(queueFailure)).toBe(true);
  });
});
