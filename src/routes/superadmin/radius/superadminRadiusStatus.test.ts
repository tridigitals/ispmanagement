import { describe, expect, it } from 'vitest';

import {
  formatManagedRadiusSessionOctets,
  getManagedRadiusSessionBadgeTone,
  getManagedRadiusSessionStatus,
  getManagedRadiusUserAttentionCount,
  getManagedRadiusUserBadgeTone,
  getManagedRadiusUserStatus,
} from './superadminRadiusStatus';

describe('superadmin managed radius status helpers', () => {
  it('treats provisioned users as healthy runtime entries', () => {
    expect(
      getManagedRadiusUserStatus({
        is_provisioned: true,
        provisioning_error: null,
      }),
    ).toBe('provisioned');
    expect(
      getManagedRadiusUserBadgeTone({
        is_provisioned: true,
        provisioning_error: null,
      }),
    ).toBe('good');
  });

  it('marks unprovisioned users with provisioning errors as needing attention', () => {
    expect(
      getManagedRadiusUserStatus({
        is_provisioned: false,
        provisioning_error: 'NAS mapping missing',
      }),
    ).toBe('not_provisioned');
    expect(
      getManagedRadiusUserBadgeTone({
        is_provisioned: false,
        provisioning_error: 'NAS mapping missing',
      }),
    ).toBe('danger');
  });

  it('counts only unprovisioned or failed users as attention items', () => {
    expect(
      getManagedRadiusUserAttentionCount([
        { is_provisioned: true, provisioning_error: null },
        { is_provisioned: false, provisioning_error: null },
        { is_provisioned: true, provisioning_error: 'stale state' },
      ]),
    ).toBe(2);
  });

  it('derives online and offline states for accounting sessions', () => {
    expect(getManagedRadiusSessionStatus({ status_type: 'start', ended_at: null })).toBe('online');
    expect(
      getManagedRadiusSessionStatus({ status_type: 'interim_update', ended_at: null }),
    ).toBe('online');
    expect(
      getManagedRadiusSessionStatus({ status_type: 'stop', ended_at: '2026-05-07T10:00:00Z' }),
    ).toBe('offline');
  });

  it('assigns muted tone to offline sessions', () => {
    expect(getManagedRadiusSessionBadgeTone({ status_type: 'start', ended_at: null })).toBe(
      'good',
    );
    expect(
      getManagedRadiusSessionBadgeTone({
        status_type: 'stop',
        ended_at: '2026-05-07T10:00:00Z',
      }),
    ).toBe('muted');
  });

  it('formats session octets into readable units', () => {
    expect(formatManagedRadiusSessionOctets(null)).toBe('—');
    expect(formatManagedRadiusSessionOctets(512)).toBe('512 B');
    expect(formatManagedRadiusSessionOctets(1536)).toBe('1.5 KB');
  });
});
