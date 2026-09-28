import { describe, expect, it } from 'vitest';

import {
  buildPhonePrefixOptions,
  composePhoneNumber,
  inferPhoneFieldState,
} from './phoneField';

describe('phoneField helpers', () => {
  it('composes full phone number from prefix and local number', () => {
    expect(composePhoneNumber('+62', '8123456789')).toBe('+628123456789');
    expect(composePhoneNumber('+1', ' 555 123 4567 ')).toBe('+15551234567');
  });

  it('infers prefix and local number from existing phone', () => {
    expect(inferPhoneFieldState('+628123456789')).toEqual({
      prefix: '+62',
      localNumber: '8123456789',
    });
    expect(inferPhoneFieldState('08123456789')).toEqual({
      prefix: '+62',
      localNumber: '08123456789',
    });
  });

  it('returns stable searchable prefix options', () => {
    const options = buildPhonePrefixOptions();

    expect(options[0]).toEqual({ value: '+62', label: 'Indonesia (+62)' });
    expect(options).toContainEqual({ value: '+1', label: 'United States (+1)' });
    expect(options.length).toBeGreaterThan(10);
  });
});
