import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('customer billing tab filters', () => {
  it('keeps the billing toolbar lightweight for a single customer', () => {
    const source = readSource('src/routes/(app)/admin/customers/[id]/CustomerBillingTab.svelte');

    expect(source).toContain('billing-stat-button');
    expect(source).toContain('onSelectBillingFilter');
    expect(source).toContain('billingFilter');

    expect(source).not.toContain("admin.customers.billing.filters.status");
    expect(source).not.toContain("$t('common.refresh') || 'Refresh'");
    expect(source).not.toContain('select class="input"');
    expect(source).not.toContain('billingQuickRange');
    expect(source).not.toContain('billingDateFrom');
    expect(source).not.toContain('billingDateTo');
    expect(source).not.toContain('onApplyQuickRange');
    expect(source).not.toContain('onBillingDateChange');
    expect(source).not.toContain('onClearFilters');
    expect(source).not.toContain("admin.customers.billing.filters.today");
    expect(source).not.toContain("admin.customers.billing.filters.from");
    expect(source).not.toContain("admin.customers.billing.filters.to");
    expect(source).not.toContain("admin.customers.billing.filters.clear");
    expect(source).not.toContain('quick-ranges');
    expect(source).not.toContain('type="date"');
  });
});
