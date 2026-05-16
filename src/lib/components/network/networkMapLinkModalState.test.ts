import { describe, expect, it } from 'vitest';

import { shouldShowManualEndpointSection } from './networkMapLinkModalState';

describe('shouldShowManualEndpointSection', () => {
  it('shows manual endpoint controls for regular create-link flow', () => {
    expect(shouldShowManualEndpointSection(null)).toBe(true);
  });

  it('hides manual endpoint controls for topology-asset connect flow', () => {
    expect(shouldShowManualEndpointSection('odp-1')).toBe(false);
  });
});
