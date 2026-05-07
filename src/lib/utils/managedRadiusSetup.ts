type SecretShape = {
  shared_secret?: string | null;
  shared_secret_masked?: string | null;
};

type ManagedRadiusPlanGateShape = {
  plan_upgrade_required?: boolean | null;
  upgrade_path?: string | null;
};

type ManagedRadiusAssignDefaultShape = {
  plan_allows_managed_radius?: boolean | null;
  tenant_has_active_assignment?: boolean | null;
  default_server_available?: boolean | null;
  can_assign_default?: boolean | null;
  can_create_mapping?: boolean | null;
};

export function getManagedRadiusDisplayedSecret(
  setup: SecretShape | null | undefined,
  revealed: boolean,
): string {
  if (!setup) return '—';
  if (revealed && setup.shared_secret) return setup.shared_secret;
  if (setup.shared_secret_masked) return setup.shared_secret_masked;
  if (setup.shared_secret) return setup.shared_secret;
  return '—';
}

export function canCopyManagedRadiusSecret(setup: SecretShape | null | undefined): boolean {
  return Boolean(setup?.shared_secret);
}

export function shouldShowManagedRadiusUpgrade(
  setup: ManagedRadiusPlanGateShape | null | undefined,
): boolean {
  return Boolean(setup?.plan_upgrade_required && setup?.upgrade_path);
}

export function shouldShowAssignDefaultManagedRadius(
  setup: ManagedRadiusAssignDefaultShape | null | undefined,
): boolean {
  return Boolean(
    setup?.plan_allows_managed_radius &&
      setup?.default_server_available &&
      !setup?.tenant_has_active_assignment &&
      setup?.can_assign_default,
  );
}

export function getManagedRadiusSummary(
  setup: (ManagedRadiusPlanGateShape &
    ManagedRadiusAssignDefaultShape & {
      configured?: boolean | null;
      assignment_endpoint_name?: string | null;
      endpoint_name?: string | null;
      default_server_available?: boolean | null;
    }) | null | undefined,
): string {
  if (!setup) return 'Managed RADIUS';
  if (setup.plan_upgrade_required) return 'Upgrade required';
  if (setup.configured) return setup.endpoint_name || 'Managed RADIUS configured';
  if (setup.tenant_has_active_assignment) {
    return setup.assignment_endpoint_name || 'Assignment active';
  }
  if (setup.default_server_available) return 'Ready to assign default endpoint';
  return 'Not configured';
}

export function shouldShowCreateManagedRadiusMapping(
  setup: ManagedRadiusAssignDefaultShape | null | undefined,
): boolean {
  return Boolean(
    setup?.plan_allows_managed_radius &&
      setup?.tenant_has_active_assignment &&
      setup?.can_create_mapping,
  );
}
