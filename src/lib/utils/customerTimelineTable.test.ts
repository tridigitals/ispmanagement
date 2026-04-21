import { describe, expect, it } from 'vitest';

import {
  buildCustomerTimelineRows,
  getCustomerTimelineActionLabel,
  getCustomerTimelineActorLabel,
  getCustomerTimelineResourceLabel,
} from './customerTimelineTable';

describe('customer timeline table helpers', () => {
  it('maps audit log labels into table rows', () => {
    expect(
      buildCustomerTimelineRows([
        {
          id: 'log-1',
          user_id: 'user-1',
          tenant_id: 'tenant-1',
          action: 'CUSTOMER_LOCATION_UPDATE',
          resource: 'customer_locations',
          resource_id: 'loc-1',
          details: 'Address updated',
          ip_address: '127.0.0.1',
          created_at: '2026-04-20T12:00:00.000Z',
          user_name: 'Admin',
        },
      ]),
    ).toEqual([
      {
        id: 'log-1',
        created_at: '2026-04-20T12:00:00.000Z',
        action: 'Location updated',
        resource: 'Location',
        actor: 'Admin',
        details: 'Address updated',
      },
    ]);
  });

  it('falls back to generic labels when values are unknown', () => {
    expect(getCustomerTimelineActionLabel('SOMETHING_NEW')).toBe('Something new');
    expect(getCustomerTimelineResourceLabel('custom_resource')).toBe('custom_resource');
    expect(
      getCustomerTimelineActorLabel({
        id: 'log-2',
        user_id: null,
        tenant_id: null,
        action: 'ANY',
        resource: 'customers',
        resource_id: null,
        details: null,
        ip_address: null,
        created_at: '2026-04-20T12:00:00.000Z',
      }),
    ).toBe('System');
  });
});
