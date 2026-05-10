import { describe, expect, it } from 'vitest';

import { resolveCustomDomainStatusView } from './customDomainStatus';

describe('resolveCustomDomainStatusView', () => {
  it('returns none when custom domain is not set', () => {
    expect(resolveCustomDomainStatusView({ customDomain: '', status: 'active' })).toEqual({
      key: 'none',
      label: 'Not set',
      tone: 'muted',
      description: 'Belum ada custom domain yang dikonfigurasi.',
    });
  });

  it('returns pending state for pending domains', () => {
    expect(
      resolveCustomDomainStatusView({
        customDomain: 'portal.customer.net',
        status: 'pending',
      }),
    ).toEqual({
      key: 'pending',
      label: 'Pending',
      tone: 'warning',
      description: 'Menunggu verifikasi atau aktivasi sebelum domain bisa dipakai.',
    });
  });

  it('returns active state for active domains', () => {
    expect(
      resolveCustomDomainStatusView({
        customDomain: 'portal.customer.net',
        status: 'active',
      }),
    ).toEqual({
      key: 'active',
      label: 'Active',
      tone: 'success',
      description: 'Domain ini sudah aktif dan dipakai untuk akses tenant.',
    });
  });

  it('prefers failure reason for failed domains', () => {
    expect(
      resolveCustomDomainStatusView({
        customDomain: 'portal.customer.net',
        status: 'failed',
        failureReason: 'DNS record belum mengarah ke target yang benar.',
      }),
    ).toEqual({
      key: 'failed',
      label: 'Failed',
      tone: 'danger',
      description: 'DNS record belum mengarah ke target yang benar.',
    });
  });
});
