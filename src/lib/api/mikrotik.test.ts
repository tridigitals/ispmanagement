import { beforeEach, describe, expect, it, vi } from 'vitest';

const getTokenOrThrow = vi.fn();
const safeInvoke = vi.fn();

vi.mock('./core', () => ({
  getTokenOrThrow,
  safeInvoke,
}));

describe('mikrotik api wrapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getTokenOrThrow.mockReturnValue('token-123');
  });

  it('applies managed radius setup through safeInvoke', async () => {
    safeInvoke.mockResolvedValue({
      configured: true,
      router_id: 'router-1',
      plan_allows_managed_radius: true,
      plan_upgrade_required: false,
      upgrade_path: null,
      tenant_has_active_assignment: true,
      default_server_available: true,
      can_assign_default: false,
      can_create_mapping: false,
      assignment_endpoint_name: 'Native RADIUS',
      endpoint_name: 'Native RADIUS',
      radius_host: 'radius.example.com',
      auth_port: 1812,
      acct_port: 1813,
      nas_ip_or_cidr: '10.10.10.1/32',
      shared_secret: null,
      shared_secret_masked: 'secr••••',
      cli_script: '/radius add ...',
      warnings: [],
    });

    const { mikrotik } = await import('./mikrotik');
    await mikrotik.routers.applyManagedRadius('router-1');

    expect(safeInvoke).toHaveBeenCalledWith('apply_mikrotik_router_managed_radius', {
      token: 'token-123',
      routerId: 'router-1',
      router_id: 'router-1',
    });
  });
});
