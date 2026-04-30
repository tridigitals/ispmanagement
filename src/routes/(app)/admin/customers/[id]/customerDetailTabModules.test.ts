import { describe, expect, it, vi } from 'vitest';

const sentinels = vi.hoisted(() => ({
  subscriptions: { name: 'customer-subscriptions-tab' },
  billing: { name: 'customer-billing-tab' },
  pppoe: { name: 'customer-pppoe-tab' },
  timeline: { name: 'customer-timeline-tab' },
}));

vi.mock('./CustomerSubscriptionsTab.svelte', () => ({
  default: sentinels.subscriptions,
}));

vi.mock('./CustomerBillingTab.svelte', () => ({
  default: sentinels.billing,
}));

vi.mock('./CustomerPppoeTab.svelte', () => ({
  default: sentinels.pppoe,
}));

vi.mock('./CustomerTimelineTab.svelte', () => ({
  default: sentinels.timeline,
}));

import {
  loadCustomerBillingTab,
  loadCustomerPppoeTab,
  loadCustomerSubscriptionsTab,
  loadCustomerTimelineTab,
} from './customerDetailTabModules';

describe('customer detail tab modules', () => {
  it('loads and caches the subscriptions tab component on demand', async () => {
    const first = await loadCustomerSubscriptionsTab();
    const second = await loadCustomerSubscriptionsTab();

    expect(first).toEqual({ default: sentinels.subscriptions });
    expect(second).toBe(first);
  });

  it('loads and caches the billing tab component on demand', async () => {
    const first = await loadCustomerBillingTab();
    const second = await loadCustomerBillingTab();

    expect(first).toEqual({ default: sentinels.billing });
    expect(second).toBe(first);
  });

  it('loads and caches the pppoe tab component on demand', async () => {
    const first = await loadCustomerPppoeTab();
    const second = await loadCustomerPppoeTab();

    expect(first).toEqual({ default: sentinels.pppoe });
    expect(second).toBe(first);
  });

  it('loads and caches the timeline tab component on demand', async () => {
    const first = await loadCustomerTimelineTab();
    const second = await loadCustomerTimelineTab();

    expect(first).toEqual({ default: sentinels.timeline });
    expect(second).toBe(first);
  });
});
