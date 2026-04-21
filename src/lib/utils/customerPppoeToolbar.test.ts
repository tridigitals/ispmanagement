import { describe, expect, it } from 'vitest';

import { getCustomerPppoeToolbarConfig } from './customerPppoeToolbar';

describe('customer PPPoE toolbar config', () => {
  it('keeps temporary create and reconcile actions hidden on customer detail', () => {
    expect(getCustomerPppoeToolbarConfig()).toEqual({
      showSearch: true,
      showRefresh: true,
      showCreate: false,
      showReconcile: false,
    });
  });
});
