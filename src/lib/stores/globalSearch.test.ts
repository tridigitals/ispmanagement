import { describe, expect, it } from 'vitest';
import { get } from 'svelte/store';

import { createGlobalSearchStore } from './globalSearch';

describe('createGlobalSearchStore', () => {
  it('tracks query, visibility, loading, and results', () => {
    const store = createGlobalSearchStore();

    store.open();
    store.setQuery('router');
    store.setLoading(true);
    store.setResults([
      {
        key: 'routers',
        label: 'Routers',
        items: [
          {
            id: 'router-1',
            kind: 'router',
            title: 'Edge Router',
            subtitle: '10.0.0.1',
            href: '/tenant-a/admin/network/routers/router-1',
            groupKey: 'routers',
            groupLabel: 'Routers',
          },
        ],
      },
    ]);

    expect(get(store).open).toBe(true);
    expect(get(store).query).toBe('router');
    expect(get(store).loading).toBe(true);
    expect(get(store).groups[0]?.items).toHaveLength(1);

    store.close();

    expect(get(store).open).toBe(false);
    expect(get(store).query).toBe('');
    expect(get(store).groups).toEqual([]);
  });
});
