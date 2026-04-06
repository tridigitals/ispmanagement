import { describe, expect, it } from 'vitest';

import { getAvailableRouterNameSuggestions } from './packageRouterMeta';

describe('package router meta helpers', () => {
  it('keeps only router-present names and removes duplicates', () => {
    expect(
      getAvailableRouterNameSuggestions([
        { id: '1', name: 'starter', router_present: true },
        { id: '2', name: 'starter', router_present: true },
        { id: '3', name: 'legacy', router_present: false },
        { id: '4', name: 'premium', router_present: true },
      ]),
    ).toEqual(['premium', 'starter']);
  });

  it('ignores blank names', () => {
    expect(
      getAvailableRouterNameSuggestions([
        { id: '1', name: ' ', router_present: true },
        { id: '2', name: '', router_present: true },
      ]),
    ).toEqual([]);
  });
});
