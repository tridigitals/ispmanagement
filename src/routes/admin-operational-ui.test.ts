import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('admin operational UI cleanup', () => {
  it('keeps high-traffic admin surfaces away from glass and decorative gradients', () => {
    const files = [
      'src/routes/(app)/admin/settings/+page.svelte',
      'src/routes/(app)/admin/team/+page.svelte',
      'src/routes/(app)/admin/support/+page.svelte',
      'src/routes/(app)/admin/services/+page.svelte',
      'src/routes/(app)/admin/backups/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toContain('--glass');
      expect(source, file).not.toContain('linear-gradient');
      expect(source, file).not.toContain('radial-gradient');
      expect(source, file).not.toContain('backdrop-filter');
    }
  });

  it('keeps admin metric grids readable on narrow mobile screens', () => {
    const files = [
      'src/routes/(app)/admin/team/+page.svelte',
      'src/routes/(app)/admin/support/+page.svelte',
      'src/routes/(app)/admin/invoices/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).toMatch(/@media \(max-width: 640px\)[\s\S]*grid-template-columns: 1fr/);
    }
  });

  it('persists admin settings tabs in the URL hash', () => {
    const source = readSource('src/routes/(app)/admin/settings/+page.svelte');

    expect(source).toContain('window.location.hash');
    expect(source).toContain('hashchange');
    expect(source).toContain('popstate');
    expect(source).toContain('selectSettingsTab');
    expect(source).toContain("new URLSearchParams(window.location.search).get('tab')");
  });

  it('uses hash links for internal admin settings tab navigation', () => {
    const source = readSource('src/routes/(app)/admin/network/incidents/+page.svelte');

    expect(source).toContain('/admin/settings#network');
    expect(source).not.toContain('/admin/settings?tab=network');
  });

  it('uses aggregate customer summary for customer index stats', () => {
    const source = readSource('src/routes/(app)/admin/customers/+page.svelte');

    expect(source).toContain('api.customers.summary()');
    expect(source).not.toContain('active: customers.filter((c) => c.is_active).length');
    expect(source).not.toContain('inactive: customers.filter((c) => !c.is_active).length');
  });

  it('keeps customer index filters, stats, service context, and quick actions operational', () => {
    const source = readSource('src/routes/(app)/admin/customers/+page.svelte');

    expect(source).toContain("let statusFilter = $state<CustomerStatusFilter>('all')");
    expect(source).toContain('syncUrlState');
    expect(source).toContain('setStatusFilter');
    expect(source).toContain("let serviceFilter = $state<CustomerServiceFilter>('all')");
    expect(source).toContain('setServiceFilter');
    expect(source).toContain("let installationFilter = $state<CustomerInstallationFilter>('all')");
    expect(source).toContain('setInstallationFilter');
    expect(source).toContain('pending_installation');
    expect(source).toContain('Pending installation');
    expect(source).toContain('customer-filter-select');
    expect(source).not.toContain('filter-segment');
    expect(source).toContain("key: 'service'");
    expect(source).toContain('serviceStatusLabel');
    expect(source).toContain("key: 'health'");
    expect(source).toContain('customerHealthLabel');
    expect(source).toContain('mobile-customer-list');
    expect(source).toContain('openAddService');
    expect(source).toContain('openCreateInvoice');
    expect(source).toContain('openWhatsApp');
  });

  it('keeps customer backend service filters set-based', () => {
    const source = readSource('src-tauri/src/services/customer_service/core.rs');
    const migrationPath =
      'src-tauri/migrations/20260504113000_add_customer_subscription_filter_indexes.up.sql';

    expect(source).toContain('WITH subscription_rollup AS');
    expect(source).not.toContain('LEFT JOIN LATERAL');
    expect(source).toContain('LEFT JOIN subscription_rollup svc');
    expect(existsSync(resolve(process.cwd(), migrationPath))).toBe(true);

    const migration = readSource(migrationPath);
    expect(migration).toContain('idx_customer_subscriptions_tenant_status_customer');
    expect(migration).toContain('ON public.customer_subscriptions(tenant_id, status, customer_id)');
  });
});
