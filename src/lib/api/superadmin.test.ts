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
      name: 'Primary',
      endpoint_host: 'radius-native.local',
      endpoint_port: 1812,
      runtime_label: 'native-runtime',
      runtime_user: 'native-radius',
      runtime_secret: 'secret',
      is_active: true,
      notes: 'Shared platform radius',
    });

    expect(safeInvoke).toHaveBeenCalledWith('create_managed_radius_server', {
      token: 'token-123',
      name: 'Primary',
      endpointHost: 'radius-native.local',
      endpoint_host: 'radius-native.local',
      endpointPort: 1812,
      endpoint_port: 1812,
      runtimeLabel: 'native-runtime',
      runtime_label: 'native-runtime',
      runtimeUser: 'native-radius',
      runtime_user: 'native-radius',
      runtimeSecret: 'secret',
      runtime_secret: 'secret',
      isActive: true,
      is_active: true,
      notes: 'Shared platform radius',
    });
  });

  it('creates a tenant radius assignment with tenant and server context', async () => {
    safeInvoke.mockResolvedValue({ ok: true, id: 'assignment-1' });

    const { superadmin } = await import('./superadmin');
    await superadmin.createManagedRadiusAssignment({
      tenant_id: 'tenant-1',
      radius_server_id: 'server-1',
      is_active: true,
    });

    expect(safeInvoke).toHaveBeenCalledWith('create_managed_radius_assignment', {
      token: 'token-123',
      tenantId: 'tenant-1',
      tenant_id: 'tenant-1',
      radiusServerId: 'server-1',
      radius_server_id: 'server-1',
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

  it('sets a managed radius server as default', async () => {
    safeInvoke.mockResolvedValue({ ok: true });

    const { superadmin } = await import('./superadmin');
    await superadmin.setManagedRadiusServerDefault('server-1');

    expect(safeInvoke).toHaveBeenCalledWith('set_managed_radius_server_default', {
      token: 'token-123',
      id: 'server-1',
    });
  });

  it('loads managed radius runtime status', async () => {
    safeInvoke.mockResolvedValue({
      enabled: true,
      running: true,
      bind_addr: '0.0.0.0',
      auth_port: 1812,
      acct_port: 1813,
      advertised_host: 'radius.example.com',
      require_message_authenticator: true,
    });

    const { superadmin } = await import('./superadmin');
    await superadmin.getManagedRadiusRuntimeStatus();

    expect(safeInvoke).toHaveBeenCalledWith('get_managed_radius_runtime_status', {
      token: 'token-123',
    });
  });

  it('lists managed radius accounting sessions through safeInvoke', async () => {
    safeInvoke.mockResolvedValue({ data: [], total: 0 });

    const { superadmin } = await import('./superadmin');
    await superadmin.listManagedRadiusSessions();

    expect(safeInvoke).toHaveBeenCalledWith('list_managed_radius_sessions', {
      token: 'token-123',
    });
  });
});
