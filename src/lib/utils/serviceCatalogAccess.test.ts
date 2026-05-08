import { describe, expect, it } from 'vitest';
import { canAccessServiceCatalog } from './serviceCatalogAccess';

describe('canAccessServiceCatalog', () => {
  it('allows superadmin when package permission exists', () => {
    expect(
      canAccessServiceCatalog({ is_super_admin: true }, false, true),
    ).toBe(true);
  });

  it('allows admin-like backoffice roles with package permission', () => {
    expect(canAccessServiceCatalog({ tenant_role: 'admin' }, true, false)).toBe(true);
    expect(canAccessServiceCatalog({ tenant_role: 'owner' }, true, false)).toBe(true);
    expect(canAccessServiceCatalog({ tenant_role: 'sales' }, true, false)).toBe(true);
    expect(canAccessServiceCatalog({ tenant_role: 'backoffice' }, false, true)).toBe(true);
  });

  it('blocks technician even when package permission exists', () => {
    expect(canAccessServiceCatalog({ tenant_role: 'technician' }, true, false)).toBe(false);
  });

  it('blocks allowed roles without package permission', () => {
    expect(canAccessServiceCatalog({ tenant_role: 'admin' }, false, false)).toBe(false);
  });
});
