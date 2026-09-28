import { describe, expect, it } from 'vitest';

import { getCustomerSearchViewState } from './customerSearchState';

describe('getCustomerSearchViewState', () => {
  it('returns idle when query is too short', () => {
    expect(getCustomerSearchViewState({ query: 'a', loading: false, hasSearched: false, resultCount: 0 }))
      .toEqual({ kind: 'idle', message: 'Search with at least 2 characters to load existing customers.' });
  });

  it('returns loading when search is in progress', () => {
    expect(getCustomerSearchViewState({ query: 'alex', loading: true, hasSearched: true, resultCount: 0 }))
      .toEqual({ kind: 'loading', message: 'Searching customers...' });
  });

  it('returns empty when search completed without results', () => {
    expect(getCustomerSearchViewState({ query: 'alex', loading: false, hasSearched: true, resultCount: 0 }))
      .toEqual({ kind: 'empty', message: 'No customers found for "alex".' });
  });

  it('returns results when there are matching customers', () => {
    expect(getCustomerSearchViewState({ query: 'alex', loading: false, hasSearched: true, resultCount: 2 }))
      .toEqual({ kind: 'results', message: '' });
  });
});
