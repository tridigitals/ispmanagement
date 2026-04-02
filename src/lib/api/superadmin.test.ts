import { beforeEach, describe, expect, it, vi } from 'vitest';

const getTokenOrThrow = vi.fn();
const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow,
  safeInvoke,
}));

describe('superadmin api wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getTokenOrThrow.mockReturnValue('token-123');
  });

  it('lists managed radius mappings through safeInvoke', async () => {
    safeInvoke.mockResolvedValue({ data: [], total: 0 });

    const { superadmin } = await import('./superadmin');
    await superadmin.listManagedRadiusMappings();

    expect(safeInvoke).toHaveBeenCalledWith('list_managed_radius_mappings', {
      token: 'token-123',
    });
  });

  it('creates a managed radius server with the expected payload', async () => {
    safeInvoke.mockResolvedValue({ ok: true, id: 'server-1' });

    const { superadmin } = await import('./superadmin');
    await superadmin.createManagedRadiusServer({
      tenant_id: 'tenant-1',
      name: 'Primary',
      db_host: 'radius-db.local',
      db_port: 5432,
      db_name: 'radius',
      db_user: 'radius',
      db_password: 'secret',
      is_active: true,
    });

    expect(safeInvoke).toHaveBeenCalledWith('create_managed_radius_server', {
      token: 'token-123',
      tenantId: 'tenant-1',
      tenant_id: 'tenant-1',
      name: 'Primary',
      dbHost: 'radius-db.local',
      db_host: 'radius-db.local',
      dbPort: 5432,
      db_port: 5432,
      dbName: 'radius',
      db_name: 'radius',
      dbUser: 'radius',
      db_user: 'radius',
      dbPassword: 'secret',
      db_password: 'secret',
      isActive: true,
      is_active: true,
    });
  });

  it('reveals a managed radius mapping secret with both id and tenant context', async () => {
    safeInvoke.mockResolvedValue({
      shared_secret: 'plain-secret',
      shared_secret_masked: 'plai••••••••cret',
    });

    const { superadmin } = await import('./superadmin');
    await superadmin.revealManagedRadiusMappingSecret('mapping-1', 'tenant-1');

    expect(safeInvoke).toHaveBeenCalledWith('reveal_managed_radius_mapping_secret', {
      token: 'token-123',
      id: 'mapping-1',
      tenantId: 'tenant-1',
      tenant_id: 'tenant-1',
    });
  });
});
