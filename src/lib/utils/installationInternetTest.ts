export type InstallationInternetTestTarget = 'router' | 'managed_radius';

type InstallationInternetTestTargetOption = {
  value: InstallationInternetTestTarget;
  label: string;
  disabled?: boolean;
};

type InstallationInternetTestTargetContext = {
  routerId?: string | null;
  managedRadiusConfigured?: boolean | null;
};

type InstallationInternetTestHintContext = {
  managedRadiusConfigured?: boolean | null;
  managedRadiusLoadError?: string | null;
  planUpgradeRequired?: boolean | null;
  tenantHasActiveAssignment?: boolean | null;
  canCreateMapping?: boolean | null;
  defaultServerAvailable?: boolean | null;
};

type InstallationInternetTestMappingRef = {
  package_id: string;
  router_id: string;
};

export function getInstallationInternetTestTargetOptions(
  context: InstallationInternetTestTargetContext,
): InstallationInternetTestTargetOption[] {
  if (!context.routerId) return [];

  return [
    { value: 'router', label: 'Router', disabled: false },
    {
      value: 'managed_radius',
      label: 'RADIUS',
      disabled: !context.managedRadiusConfigured,
    },
  ];
}

export function normalizeInstallationInternetTestTarget(
  current: InstallationInternetTestTarget,
  options: InstallationInternetTestTargetOption[],
): InstallationInternetTestTarget {
  if (options.some((option) => option.value === current && !option.disabled)) return current;
  return options.find((option) => !option.disabled)?.value || 'router';
}

export function getInstallationInternetTestTargetHint(
  context: InstallationInternetTestHintContext,
): string {
  if (context.managedRadiusLoadError) {
    return 'Managed RADIUS setup could not be loaded. Check permissions or router setup.';
  }
  if (context.planUpgradeRequired) {
    return 'Managed RADIUS feature is not enabled for this tenant yet.';
  }
  if (!context.tenantHasActiveAssignment && context.defaultServerAvailable) {
    return 'Managed RADIUS tenant assignment is not active yet.';
  }
  if (context.tenantHasActiveAssignment && context.canCreateMapping) {
    return 'Managed RADIUS NAS mapping for this router is not active yet.';
  }
  if (!context.managedRadiusConfigured) {
    return 'Managed RADIUS is not configured for this router yet';
  }
  return '';
}

export function resolveInstallationInternetTestRouterId(context: {
  explicitRouterId?: string | null;
  packageId?: string | null;
  mappings: InstallationInternetTestMappingRef[];
}): string {
  const explicitRouterId = `${context.explicitRouterId || ''}`.trim();
  if (explicitRouterId) return explicitRouterId;

  const packageId = `${context.packageId || ''}`.trim();
  if (!packageId) return '';

  const routerIds = Array.from(
    new Set(
      context.mappings
        .filter((item) => item.package_id === packageId)
        .map((item) => `${item.router_id || ''}`.trim())
        .filter(Boolean),
    ),
  );

  return routerIds.length === 1 ? routerIds[0] : '';
}
