import { describe, expect, it } from 'vitest';

import {
  buildBackofficeInstallationOrderPayload,
  inferInitialCustomerMode,
  type OrderWizardDraft,
} from './orderWizardState';

function baseDraft(): OrderWizardDraft {
  return {
    customerMode: 'new',
    existingCustomerId: '',
    customer: {
      name: 'Jane Doe',
      email: 'jane@example.com',
      phone: '',
      notes: '',
      is_active: true,
    },
    locationMode: 'new',
    existingLocationId: '',
    location: {
      label: 'Rumah',
      address_line1: 'Jl. Merdeka 1',
      address_line2: '',
      city: 'Jakarta',
      state: '',
      postal_code: '',
      country: 'ID',
      latitude: '',
      longitude: '',
      notes: '',
    },
    packageId: 'pkg-1',
    billingCycle: 'monthly',
    notes: '',
    requestedInstallationDate: '',
  };
}

describe('order wizard state helpers', () => {
  it('prefills existing-customer mode when a customer id is provided', () => {
    expect(inferInitialCustomerMode('cust-1')).toBe('existing');
    expect(inferInitialCustomerMode('')).toBe('new');
  });

  it('builds payload for a new customer and new location flow', () => {
    const payload = buildBackofficeInstallationOrderPayload(baseDraft());

    expect(payload.customer_mode).toBe('new');
    expect(payload.customer?.name).toBe('Jane Doe');
    expect(payload.location_mode).toBe('new');
    expect(payload.location?.label).toBe('Rumah');
    expect(payload.package_id).toBe('pkg-1');
    expect(payload.customer_id).toBeNull();
    expect(payload.location_id).toBeNull();
  });

  it('builds payload for an existing customer and existing location flow', () => {
    const payload = buildBackofficeInstallationOrderPayload({
      ...baseDraft(),
      customerMode: 'existing',
      existingCustomerId: 'cust-1',
      locationMode: 'existing',
      existingLocationId: 'loc-1',
    });

    expect(payload.customer_mode).toBe('existing');
    expect(payload.customer_id).toBe('cust-1');
    expect(payload.customer).toBeNull();
    expect(payload.location_mode).toBe('existing');
    expect(payload.location_id).toBe('loc-1');
    expect(payload.location).toBeNull();
  });

  it('rejects missing existing-customer and address selections', () => {
    expect(() =>
      buildBackofficeInstallationOrderPayload({
        ...baseDraft(),
        customerMode: 'existing',
        locationMode: 'existing',
      }),
    ).toThrow('Existing customer is required');

    expect(() =>
      buildBackofficeInstallationOrderPayload({
        ...baseDraft(),
        customerMode: 'existing',
        existingCustomerId: 'cust-1',
        locationMode: 'existing',
      }),
    ).toThrow('Existing location is required');
  });
});
