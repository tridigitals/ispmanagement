import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  billingPlanPanel: { name: 'tenant-billing-plan-panel' },
  emailTab: { name: 'settings-email-tab' },
  paymentTab: { name: 'settings-payment-tab' },
  serviceTab: { name: 'settings-service-tab' },
}));

vi.mock('$lib/components/billing/TenantBillingPlanPanel.svelte', () => ({
  default: sentinels.billingPlanPanel,
}));

vi.mock('./SettingsEmailTab.svelte', () => ({
  default: sentinels.emailTab,
}));

vi.mock('./SettingsPaymentTab.svelte', () => ({
  default: sentinels.paymentTab,
}));

vi.mock('./SettingsServiceTab.svelte', () => ({
  default: sentinels.serviceTab,
}));

import {
  loadAdminSettingsEmailTab,
  loadAdminSettingsPaymentTab,
  loadAdminSettingsServiceTab,
  loadTenantBillingPlanPanel,
} from './adminSettingsPageModules';

describe('admin settings page modules', () => {
  it('loads and caches the tenant billing plan panel lazily', async () => {
    const first = await loadTenantBillingPlanPanel();
    const second = await loadTenantBillingPlanPanel();

    expect(first.BillingPlanPanelComponent).toBe(sentinels.billingPlanPanel);
    expect(second).toBe(first);
  });

  it('loads and caches the email tab lazily', async () => {
    const first = await loadAdminSettingsEmailTab();
    const second = await loadAdminSettingsEmailTab();

    expect(first.SettingsEmailTabComponent).toBe(sentinels.emailTab);
    expect(second).toBe(first);
  });

  it('loads and caches the payment tab lazily', async () => {
    const first = await loadAdminSettingsPaymentTab();
    const second = await loadAdminSettingsPaymentTab();

    expect(first.SettingsPaymentTabComponent).toBe(sentinels.paymentTab);
    expect(second).toBe(first);
  });

  it('loads and caches the service tab lazily', async () => {
    const first = await loadAdminSettingsServiceTab();
    const second = await loadAdminSettingsServiceTab();

    expect(first.SettingsServiceTabComponent).toBe(sentinels.serviceTab);
    expect(second).toBe(first);
  });
});
