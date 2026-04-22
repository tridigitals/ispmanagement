import { describe, expect, it } from 'vitest';

import { getLucideIconImportPath } from './iconResolver';

describe('icon resolver', () => {
  it('maps known aliases to lucide icon module names', () => {
    expect(getLucideIconImportPath('dashboard')).toBe('layout-dashboard');
    expect(getLucideIconImportPath('alert')).toBe('alert-circle');
    expect(getLucideIconImportPath('ban')).toBe('circle-off');
    expect(getLucideIconImportPath('trash')).toBe('trash-2');
    expect(getLucideIconImportPath('sidebar-toggle')).toBe('panel-left');
  });

  it('passes through direct lucide names and only defaults when empty', () => {
    expect(getLucideIconImportPath('mail')).toBe('mail');
    expect(getLucideIconImportPath('not-a-real-icon')).toBe('not-a-real-icon');
    expect(getLucideIconImportPath(undefined)).toBe('help-circle');
  });
});
