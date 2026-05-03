import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const css = readFileSync(resolve(process.cwd(), 'src/lib/styles/global.css'), 'utf8');

function rootToken(name: string) {
  return new RegExp(`--${name}\\s*:`).test(css);
}

describe('global design tokens', () => {
  it('defines semantic aliases used by app pages', () => {
    expect(rootToken('color-primary-light')).toBe(true);
    expect(rootToken('primary')).toBe(true);
    expect(rootToken('primary-color')).toBe(true);
    expect(rootToken('bg-success')).toBe(true);
    expect(rootToken('text-success')).toBe(true);
    expect(rootToken('bg-card')).toBe(true);
    expect(rootToken('bg-input')).toBe(true);
    expect(rootToken('border-primary')).toBe(true);
    expect(rootToken('border-radius')).toBe(true);
    expect(rootToken('shadow-lg')).toBe(true);
    expect(rootToken('color-primary-transparent')).toBe(true);
  });
});
