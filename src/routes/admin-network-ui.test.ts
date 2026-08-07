import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function readSource(path: string) {
  return readFileSync(resolve(process.cwd(), path), 'utf8');
}

describe('admin network UI cleanup', () => {
  it('uses clean shared surface tokens on top-level network operation pages', () => {
    const files = [
      'src/routes/(app)/admin/network/alerts/+page.svelte',
      'src/routes/(app)/admin/network/incidents/+page.svelte',
      'src/routes/(app)/admin/network/installations/+page.svelte',
      'src/routes/(app)/admin/network/dhcp-static/+page.svelte',
      'src/routes/(app)/admin/network/ip-pools/+page.svelte',
      'src/routes/(app)/admin/network/noc/+page.svelte',
      'src/routes/(app)/admin/network/ppp-profiles/+page.svelte',
      'src/routes/(app)/admin/network/pppoe/+page.svelte',
      'src/routes/(app)/admin/network/routers/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).not.toContain('var(--bg-card)');
      expect(source, file).not.toContain('border-radius: 18px');
      expect(source, file).not.toContain('0 12px 30px rgba(0, 0, 0, 0.2)');
      expect(source, file).toContain('var(--bg-surface)');
      expect(source, file).toContain('var(--radius-lg)');
    }
  });

  it('keeps network metric grids readable on mobile', () => {
    const files = [
      'src/routes/(app)/admin/network/alerts/+page.svelte',
      'src/routes/(app)/admin/network/installations/+page.svelte',
      'src/routes/(app)/admin/network/dhcp-static/+page.svelte',
      'src/routes/(app)/admin/network/noc/+page.svelte',
      'src/routes/(app)/admin/network/pppoe/+page.svelte',
      'src/routes/(app)/admin/network/routers/+page.svelte',
    ];

    for (const file of files) {
      const source = readSource(file);

      expect(source, file).toMatch(/@media \(max-width: 640px\)[\s\S]*grid-template-columns: 1fr/);
    }
  });

  it('keeps DHCP static admin page capable of direct create and edit flows', () => {
    const source = readSource('src/routes/(app)/admin/network/dhcp-static/+page.svelte');

    expect(source).toContain('Create DHCP Static');
    expect(source).toContain('Edit DHCP Static');
    expect(source).toContain('submitCreate');
    expect(source).toContain('submitEdit');
    expect(source).toContain('api.dhcpStatic.services.create');
    expect(source).toContain('api.dhcpStatic.services.update');
  });

  it('reuses DHCP static validation helpers in admin and installation flows', () => {
    const adminSource = readSource('src/routes/(app)/admin/network/dhcp-static/+page.svelte');
    const installationSource = readSource('src/routes/(app)/admin/network/installations/+page.svelte');

    for (const source of [adminSource, installationSource]) {
      expect(source).toContain('normalizeDhcpStaticMacAddress');
      expect(source).toContain('validateDhcpStaticIpv4Address');
      expect(source).toContain('validateDhcpStaticQueueRateLimit');
    }
  });

  it('derives DHCP static queue presets from package context', () => {
    const adminSource = readSource('src/routes/(app)/admin/network/dhcp-static/+page.svelte');
    const installationSource = readSource('src/routes/(app)/admin/network/installations/+page.svelte');

    expect(adminSource).toContain('buildDhcpStaticQueueRateLimitPresets');
    expect(installationSource).toContain('buildDhcpStaticQueueRateLimitPresets');
    expect(adminSource).toContain('queueRateLimitPresets[0]');
    expect(installationSource).toContain('installationDhcpQueueRateLimitPresets[0]');
  });

  it('shows DHCP provisioning state and blocks completion until the lease is ready', () => {
    const installationSource = readSource('src/routes/(app)/admin/network/installations/+page.svelte');
    const modalSource = readSource('src/routes/(app)/admin/network/installations/InstallationDetailModal.svelte');

    expect(installationSource).toContain('getDhcpStaticProvisioningStatus');
    expect(installationSource).toContain('isDhcpStaticProvisioningReady');
    expect(installationSource).toContain('refreshInstallationDhcpService');
    expect(modalSource).toContain('provisioning_completion_blocked');
    expect(modalSource).toContain('installationDhcpProvisioningError');
  });

  it('exposes installation visibility controls on the installations page', () => {
    const source = readSource('src/routes/(app)/admin/network/installations/+page.svelte');

    expect(source).toContain('installation_work_order_visibility_mode');
    expect(source).toContain('admin_only');
    expect(source).toContain('all_staff');
    expect(source).toContain('Work Order Visibility');
  });

  it('shows the active installation visibility mode in the page header', () => {
    const source = readSource('src/routes/(app)/admin/network/installations/+page.svelte');

    expect(source).toContain('visibility-mode-pill');
    expect(source).toContain('visibilityModeLabel');
    expect(source).toContain('visibilityModeHint');
  });
});
