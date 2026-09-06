import { describe, expect, it } from 'vitest';
import { ipPoolNextPoolOptions } from './ipPoolOptions';

describe('ipPoolNextPoolOptions', () => {
  it('unik + sortir + kecualikan current', () => {
    expect(ipPoolNextPoolOptions(['b', 'a', 'b', null, ''], 'a')).toEqual(['b']);
    expect(ipPoolNextPoolOptions(['x', 'y'], '')).toEqual(['x', 'y']);
  });
});
