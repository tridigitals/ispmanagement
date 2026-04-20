import { describe, expect, it } from 'vitest';

import { getLucideIconModuleLoader, hasLucideIconModule } from './iconModules';

describe('icon module registry', () => {
  it('exposes loaders for icons used by the app shell', () => {
    expect(hasLucideIconModule('mail')).toBe(true);
    expect(hasLucideIconModule('lock')).toBe(true);
    expect(hasLucideIconModule('eye')).toBe(true);
    expect(typeof getLucideIconModuleLoader('mail')).toBe('function');
  });

  it('returns no loader for icons outside the registry', () => {
    expect(hasLucideIconModule('not-a-real-icon')).toBe(false);
    expect(getLucideIconModuleLoader('not-a-real-icon')).toBeUndefined();
  });
});
