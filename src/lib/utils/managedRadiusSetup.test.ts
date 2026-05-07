import { describe, expect, it } from 'vitest';

import {
  canCopyManagedRadiusSecret,
  getManagedRadiusSummary,
  getManagedRadiusDisplayedSecret,
  shouldShowCreateManagedRadiusMapping,
  shouldShowAssignDefaultManagedRadius,
  shouldShowManagedRadiusUpgrade,
} from './managedRadiusSetup';

describe('managed radius setup helpers', () => {
  it('shows masked secret by default', () => {
    expect(
      getManagedRadiusDisplayedSecret(
        {
          shared_secret: 'secret-clear',
          shared_secret_masked: 'secr••••••••lear',
        },
        false,
      ),
    ).toBe('secr••••••••lear');
  });

  it('shows clear secret when reveal is enabled', () => {
    expect(
      getManagedRadiusDisplayedSecret(
        {
          shared_secret: 'secret-clear',
          shared_secret_masked: 'secr••••••••lear',
        },
        true,
      ),
    ).toBe('secret-clear');
  });

  it('returns placeholder when secret is missing', () => {
    expect(
      getManagedRadiusDisplayedSecret(
        {
          shared_secret: null,
          shared_secret_masked: null,
        },
        true,
      ),
    ).toBe('—');
  });

  it('only allows copy when clear secret exists', () => {
    expect(canCopyManagedRadiusSecret({ shared_secret: 'abc' })).toBe(true);
    expect(canCopyManagedRadiusSecret({ shared_secret: null })).toBe(false);
  });

  it('shows upgrade state when plan requires an upgrade', () => {
    expect(
      shouldShowManagedRadiusUpgrade({
        plan_upgrade_required: true,
        upgrade_path: '/admin/subscription',
      }),
    ).toBe(true);
    expect(
      shouldShowManagedRadiusUpgrade({
        plan_upgrade_required: false,
        upgrade_path: '/admin/subscription',
      }),
    ).toBe(false);
  });

  it('shows assign-default action only when eligible', () => {
    expect(
      shouldShowAssignDefaultManagedRadius({
        plan_allows_managed_radius: true,
        tenant_has_active_assignment: false,
        default_server_available: true,
        can_assign_default: true,
      }),
    ).toBe(true);
    expect(
      shouldShowAssignDefaultManagedRadius({
        plan_allows_managed_radius: true,
        tenant_has_active_assignment: true,
        default_server_available: true,
        can_assign_default: false,
      }),
    ).toBe(false);
  });

  it('builds a compact summary for the trigger button', () => {
    expect(
      getManagedRadiusSummary({
        configured: true,
        endpoint_name: 'Primary Radius',
      }),
    ).toBe('Primary Radius');
    expect(
      getManagedRadiusSummary({
        plan_upgrade_required: true,
      }),
    ).toBe('Upgrade required');
  });

  it('shows create-mapping action when assignment is active but mapping is still missing', () => {
    expect(
      shouldShowCreateManagedRadiusMapping({
        plan_allows_managed_radius: true,
        tenant_has_active_assignment: true,
        can_create_mapping: true,
      }),
    ).toBe(true);
    expect(
      shouldShowCreateManagedRadiusMapping({
        plan_allows_managed_radius: true,
        tenant_has_active_assignment: false,
        can_create_mapping: true,
      }),
    ).toBe(false);
  });
});
