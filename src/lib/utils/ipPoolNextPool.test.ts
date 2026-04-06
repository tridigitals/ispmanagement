import { describe, expect, it } from 'vitest';

import {
  getAvailableNextPoolNames,
  getInitialNextPoolFieldState,
  resolveNextPoolFieldValue,
} from './ipPoolNextPool';

describe('ip pool next-pool helpers', () => {
  it('filters out the current pool from selectable next-pool options', () => {
    expect(
      getAvailableNextPoolNames(
        [
          { name: 'pool-a' },
          { name: 'pool-b' },
          { name: 'pool-c' },
        ],
        'pool-b',
      ),
    ).toEqual(['pool-a', 'pool-c']);
  });

  it('uses select mode when the current next-pool exists in mirrored rows', () => {
    expect(
      getInitialNextPoolFieldState(
        [
          { name: 'pool-a' },
          { name: 'pool-b' },
        ],
        'pool-b',
      ),
    ).toEqual({
      mode: 'select',
      selectedValue: 'pool-b',
      manualValue: '',
    });
  });

  it('falls back to manual mode when the current next-pool is not in the mirrored rows', () => {
    expect(
      getInitialNextPoolFieldState(
        [
          { name: 'pool-a' },
          { name: 'pool-b' },
        ],
        'pool-z',
      ),
    ).toEqual({
      mode: 'manual',
      selectedValue: '',
      manualValue: 'pool-z',
    });
  });

  it('normalizes select and manual values into the payload field', () => {
    expect(resolveNextPoolFieldValue({ mode: 'select', selectedValue: ' pool-a ', manualValue: '' })).toBe(
      'pool-a',
    );
    expect(
      resolveNextPoolFieldValue({ mode: 'manual', selectedValue: '', manualValue: ' custom-pool ' }),
    ).toBe('custom-pool');
    expect(resolveNextPoolFieldValue({ mode: 'select', selectedValue: '', manualValue: '' })).toBeNull();
  });
});
