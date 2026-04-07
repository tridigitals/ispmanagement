import { describe, expect, it } from 'vitest';

import { shouldAllowInstallationInvoiceCreation } from './installationInvoice';

describe('installation invoice helpers', () => {
  it('allows invoice creation while awaiting first payment and no invoice exists', () => {
    expect(
      shouldAllowInstallationInvoiceCreation({
        workOrderStatus: 'completed',
        subscriptionStatus: 'pending_installation',
        hasCustomerPackageInvoice: false,
      }),
    ).toBe(true);
  });

  it('allows invoice recreation during grace period when the first invoice is still missing', () => {
    expect(
      shouldAllowInstallationInvoiceCreation({
        workOrderStatus: 'completed',
        subscriptionStatus: 'grace_active',
        hasCustomerPackageInvoice: false,
      }),
    ).toBe(true);
  });

  it('blocks invoice creation when a package invoice already exists', () => {
    expect(
      shouldAllowInstallationInvoiceCreation({
        workOrderStatus: 'completed',
        subscriptionStatus: 'grace_active',
        hasCustomerPackageInvoice: true,
      }),
    ).toBe(false);
  });

  it('blocks invoice creation for non-completed work orders', () => {
    expect(
      shouldAllowInstallationInvoiceCreation({
        workOrderStatus: 'in_progress',
        subscriptionStatus: 'pending_installation',
        hasCustomerPackageInvoice: false,
      }),
    ).toBe(false);
  });
});
