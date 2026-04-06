export type IpPoolCrudGateState = {
  blocked: boolean;
  reason: 'router_required' | null;
};

export type IpPoolDependencyCounts = {
  pppoe_accounts?: number | null;
  isp_package_router_mappings?: number | null;
};

export type IpPoolDeleteState =
  | {
      status: 'warning';
      warning: true;
      allowed: true;
      dependencyCounts: Required<IpPoolDependencyCounts>;
      totalDependencies: number;
    }
  | {
      status: 'clean';
      warning: false;
      allowed: true;
      dependencyCounts: Required<IpPoolDependencyCounts>;
      totalDependencies: number;
    };

export type IpPoolMutationErrorState = {
  code: 'mirror_sync_failed' | 'router_write_failed';
  tone: 'warning' | 'error';
  title: string;
  message: string;
};

function normalizeCount(value: number | null | undefined): number {
  const normalized = Math.floor(Number(value ?? 0));
  return Number.isFinite(normalized) && normalized > 0 ? normalized : 0;
}

export function getIpPoolCrudGateState(routerId: string | null | undefined): IpPoolCrudGateState {
  const hasRouter = Boolean(String(routerId ?? '').trim());

  return hasRouter
    ? {
        blocked: false,
        reason: null,
      }
    : {
        blocked: true,
        reason: 'router_required',
      };
}

export function getIpPoolDeleteState(
  dependencyCounts: IpPoolDependencyCounts | null | undefined,
): IpPoolDeleteState {
  const normalizedDependencyCounts = {
    pppoe_accounts: normalizeCount(dependencyCounts?.pppoe_accounts),
    isp_package_router_mappings: normalizeCount(dependencyCounts?.isp_package_router_mappings),
  };

  const totalDependencies =
    normalizedDependencyCounts.pppoe_accounts + normalizedDependencyCounts.isp_package_router_mappings;

  if (totalDependencies > 0) {
    return {
      status: 'warning',
      warning: true,
      allowed: true,
      dependencyCounts: normalizedDependencyCounts,
      totalDependencies,
    };
  }

  return {
    status: 'clean',
    warning: false,
    allowed: true,
    dependencyCounts: normalizedDependencyCounts,
    totalDependencies,
  };
}

export function getIpPoolMutationErrorState(code: string | null | undefined): IpPoolMutationErrorState {
  if (code === 'mirror_sync_failed') {
    return {
      code: 'mirror_sync_failed',
      tone: 'warning',
      title: 'RouterOS changed, but the local mirror could not be refreshed',
      message: 'The IP pool was written on the router, but the mirrored database cache could not be refreshed yet.',
    };
  }

  return {
    code: 'router_write_failed',
    tone: 'error',
    title: 'RouterOS rejected the IP pool change',
    message: 'The IP pool was not changed on the router. Fix the router error and try again.',
  };
}

export function isIpPoolStaleTargetConflict(message: string | null | undefined): boolean {
  const normalized = String(message ?? '').toLowerCase();
  return normalized.includes('ip pool no longer exists on router');
}
