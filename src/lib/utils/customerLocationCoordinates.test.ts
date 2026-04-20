import { describe, expect, it } from 'vitest';

import {
  formatLocationCoordinates,
  parseOptionalCoordinateInput,
  validateOptionalCoordinates,
} from './customerLocationCoordinates';

describe('customer location coordinates helpers', () => {
  it('parses blank input as null', () => {
    expect(parseOptionalCoordinateInput('')).toBeNull();
    expect(parseOptionalCoordinateInput('   ')).toBeNull();
  });

  it('detects invalid coordinate pairs', () => {
    expect(validateOptionalCoordinates('-7.2', '')).toEqual({
      latitude: null,
      longitude: null,
      error: 'both_required',
    });
    expect(validateOptionalCoordinates('abc', '110.1')).toEqual({
      latitude: null,
      longitude: null,
      error: 'invalid_number',
    });
  });

  it('validates range and returns parsed coordinates', () => {
    expect(validateOptionalCoordinates('-91', '110')).toEqual({
      latitude: null,
      longitude: null,
      error: 'latitude_range',
    });
    expect(validateOptionalCoordinates('-7.275233', '110.355211')).toEqual({
      latitude: -7.275233,
      longitude: 110.355211,
      error: null,
    });
  });

  it('formats coordinates for table display', () => {
    expect(formatLocationCoordinates(-7.275233, 110.355211)).toBe('-7.275233, 110.355211');
    expect(formatLocationCoordinates(null, 110.355211)).toBeNull();
  });
});
