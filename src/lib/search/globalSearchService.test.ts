import { describe, expect, it, vi } from 'vitest';

import { searchGlobalTopbar } from './globalSearchService';
import type { GlobalSearchProvider, GlobalSearchProviderContext } from './globalSearchModel';

const context: GlobalSearchProviderContext = {
  can: () => true,
  isSuperAdmin: false,
  shellScope: 'admin',
  tenantPrefix: '/tenant-a',
};

describe('searchGlobalTopbar', () => {
  it('trims queries, skips disabled providers, and groups results by provider order', async () => {
    const disabledSearch = vi.fn();
    const providers: GlobalSearchProvider[] = [
      {
        key: 'customers',
        label: 'Customers',
        isEnabled: () => true,
        search: vi.fn(async (query: string) => [
          {
            id: 'customer-1',
            kind: 'customer' as const,
            title: `Customer ${query}`,
            subtitle: 'Alpha',
            href: '/tenant-a/admin/customers/customer-1',
            groupKey: 'customers',
            groupLabel: 'Customers',
          },
        ]),
      },
      {
        key: 'routers',
        label: 'Routers',
        isEnabled: () => false,
        search: disabledSearch,
      },
      {
        key: 'invoices',
        label: 'Invoices',
        isEnabled: () => true,
        search: vi.fn(async () => [
          {
            id: 'invoice-1',
            kind: 'invoice' as const,
            title: 'INV-001',
            subtitle: 'Pending',
            href: '/tenant-a/admin/invoices/invoice-1',
            groupKey: 'invoices',
            groupLabel: 'Invoices',
          },
        ]),
      },
    ];

    const result = await searchGlobalTopbar('  alpha  ', context, providers);

    expect(result.query).toBe('alpha');
    expect(result.groups.map((group) => group.key)).toEqual(['customers', 'invoices']);
    expect(result.groups[0]?.items[0]?.title).toBe('Customer alpha');
    expect(disabledSearch).not.toHaveBeenCalled();
  });

  it('ranks exact and prefix title matches ahead of loose subtitle matches within a group', async () => {
    const providers: GlobalSearchProvider[] = [
      {
        key: 'customers',
        label: 'Customers',
        isEnabled: () => true,
        search: vi.fn(async () => [
          {
            id: 'customer-subtitle',
            kind: 'customer' as const,
            title: 'Beta Network',
            subtitle: 'alpha project',
            href: '/tenant-a/admin/customers/customer-subtitle',
            groupKey: 'customers',
            groupLabel: 'Customers',
          },
          {
            id: 'customer-prefix',
            kind: 'customer' as const,
            title: 'Alpha Fiber',
            subtitle: 'Prefix match',
            href: '/tenant-a/admin/customers/customer-prefix',
            groupKey: 'customers',
            groupLabel: 'Customers',
          },
          {
            id: 'customer-exact',
            kind: 'customer' as const,
            title: 'alpha',
            subtitle: 'Exact match',
            href: '/tenant-a/admin/customers/customer-exact',
            groupKey: 'customers',
            groupLabel: 'Customers',
          },
        ]),
      },
    ];

    const result = await searchGlobalTopbar('alpha', context, providers);

    expect(result.groups[0]?.items.map((item) => item.id)).toEqual([
      'customer-exact',
      'customer-prefix',
      'customer-subtitle',
    ]);
  });

  it('skips providers that require longer queries', async () => {
    const shortQueryProviderSearch = vi.fn();
    const providers: GlobalSearchProvider[] = [
      {
        key: 'routers',
        label: 'Routers',
        isEnabled: () => true,
        minQueryLength: 2,
        search: shortQueryProviderSearch,
      },
    ];

    const result = await searchGlobalTopbar('a', context, providers);

    expect(result.groups).toEqual([]);
    expect(shortQueryProviderSearch).not.toHaveBeenCalled();
  });
});
