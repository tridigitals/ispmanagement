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

type InstallationWorkOrderSubscriptionRef = {
  id: string;
  tenant_id: string;
  subscription_id: string;
  customer_id: string;
  location_id: string;
  package_id?: string | null;
  router_id?: string | null;
  package_name?: string | null;
  location_label?: string | null;
  router_name?: string | null;
  subscription_status?: string | null;
  subscription_starts_at?: string | null;
  subscription_grace_until?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export function buildInstallationSubscriptionFallback(row: InstallationWorkOrderSubscriptionRef) {
  const packageId = `${row.package_id || ''}`.trim();
  if (!packageId) return null;

  const timestamp = row.updated_at || row.created_at || new Date(0).toISOString();

  return {
    id: row.subscription_id,
    tenant_id: row.tenant_id,
    customer_id: row.customer_id,
    location_id: row.location_id,
    package_id: packageId,
    router_id: row.router_id || null,
    billing_cycle: '',
    price: 0,
    currency_code: '',
    status: row.subscription_status || 'pending_installation',
    starts_at: row.subscription_starts_at || null,
    ends_at: null,
    grace_started_at: null,
    grace_until: row.subscription_grace_until || null,
    notes: null,
    created_at: row.created_at || timestamp,
    updated_at: row.updated_at || timestamp,
    package_name: row.package_name || null,
    location_label: row.location_label || null,
    router_name: row.router_name || null,
    latest_work_order_id: row.id,
    latest_work_order_status: null,
    can_request_reopen: false,
    latest_reschedule_status: null,
    latest_reschedule_requested_at: null,
  };
}

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
