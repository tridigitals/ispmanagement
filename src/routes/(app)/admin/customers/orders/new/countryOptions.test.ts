import { describe, expect, it } from 'vitest';

import { buildCountryOptions } from './countryOptions';

describe('buildCountryOptions', () => {
  it('builds searchable country options with Indonesia first', () => {
    const options = buildCountryOptions();

    expect(options[0]).toEqual({
      value: 'ID',
      label: 'Indonesia (ID)',
    });
    expect(options).toContainEqual({
      value: 'US',
      label: 'United States (US)',
    });
    expect(options.length).toBeGreaterThan(10);
  });
});
