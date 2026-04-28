import { describe, expect, it } from 'vitest';

import { getLucideIconModuleLoader, hasLucideIconModule } from './iconModules';

describe('icon module registry', () => {
  it('exposes loaders for icons used by the app shell', () => {
    expect(hasLucideIconModule('mail')).toBe(true);
    expect(hasLucideIconModule('lock')).toBe(true);
    expect(hasLucideIconModule('eye')).toBe(true);
    expect(hasLucideIconModule('sun')).toBe(true);
    expect(hasLucideIconModule('moon')).toBe(true);
    expect(hasLucideIconModule('user-check')).toBe(true);
    expect(hasLucideIconModule('square-pen')).toBe(true);
    expect(hasLucideIconModule('circle-off')).toBe(true);
    expect(hasLucideIconModule('circle')).toBe(true);
    expect(hasLucideIconModule('webhook')).toBe(true);
    expect(typeof getLucideIconModuleLoader('mail')).toBe('function');
    expect(typeof getLucideIconModuleLoader('sun')).toBe('function');
    expect(typeof getLucideIconModuleLoader('moon')).toBe('function');
    expect(typeof getLucideIconModuleLoader('user-check')).toBe('function');
    expect(typeof getLucideIconModuleLoader('square-pen')).toBe('function');
    expect(typeof getLucideIconModuleLoader('circle-off')).toBe('function');
    expect(typeof getLucideIconModuleLoader('circle')).toBe('function');
    expect(typeof getLucideIconModuleLoader('webhook')).toBe('function');
  });

  it('returns no loader for icons outside the registry', () => {
    expect(hasLucideIconModule('not-a-real-icon')).toBe(false);
    expect(getLucideIconModuleLoader('not-a-real-icon')).toBeUndefined();
  });
});
