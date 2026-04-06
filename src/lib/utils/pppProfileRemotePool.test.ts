import { describe, expect, it } from 'vitest';

import {
  getPppProfileRemotePoolOptions,
  getPppProfileRemotePoolValue,
} from './pppProfileRemotePool';

describe('ppp profile remote pool helpers', () => {
  it('maps ip pool rows into sorted unique remote options', () => {
    expect(
      getPppProfileRemotePoolOptions([
        { name: 'pool-b' },
        { name: 'pool-a' },
        { name: 'pool-b' },
      ]),
    ).toEqual(['pool-a', 'pool-b']);
  });

  it('keeps remote value only when it still exists in the available pool list', () => {
    expect(getPppProfileRemotePoolValue(['pool-a', 'pool-b'], 'pool-b')).toBe('pool-b');
    expect(getPppProfileRemotePoolValue(['pool-a', 'pool-b'], 'pool-z')).toBe('');
    expect(getPppProfileRemotePoolValue(['pool-a', 'pool-b'], '')).toBe('');
  });
});
