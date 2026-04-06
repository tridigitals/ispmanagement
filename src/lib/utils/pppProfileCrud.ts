export type PppProfileCrudGateState = {
  blocked: boolean;
  reason: 'router_required' | null;
};

export type PppProfileDependencyCounts = {
  pppoe_accounts?: number | null;
  isp_package_router_mappings?: number | null;
};

export type PppProfileDeleteState =
  | {
      status: 'blocked';
      blocked: true;
      allowed: false;
      dependencyCounts: Required<PppProfileDependencyCounts>;
      totalDependencies: number;
    }
  | {
      status: 'allowed';
      blocked: false;
      allowed: true;
      dependencyCounts: Required<PppProfileDependencyCounts>;
      totalDependencies: number;
    };

export type PppProfileMutationErrorState = {
  code: 'mirror_sync_failed' | 'router_write_failed';
  tone: 'warning' | 'error';
  title: string;
  message: string;
};

export type PppProfileOnlyOneState = {
  enabled: boolean;
};

export type PppProfileFormPayloadInput = {
  name: string;
  local_address: string;
  remote_address: string;
  rate_limit: string;
  dns_server: string;
  comment: string;
  only_one: boolean;
};

export type PppProfileNormalizedPayload = {
  name: string;
  local_address: string | null;
  remote_address: string | null;
  rate_limit: string | null;
  dns_server: string | null;
  comment: string | null;
  only_one: boolean;
};

function normalizeCount(value: number | null | undefined): number {
  const normalized = Math.floor(Number(value ?? 0));
  return Number.isFinite(normalized) && normalized > 0 ? normalized : 0;
}

export function getPppProfileCrudGateState(routerId: string | null | undefined): PppProfileCrudGateState {
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

export function getPppProfileDeleteState(
  dependencyCounts: PppProfileDependencyCounts | null | undefined,
): PppProfileDeleteState {
  const normalizedDependencyCounts = {
    pppoe_accounts: normalizeCount(dependencyCounts?.pppoe_accounts),
    isp_package_router_mappings: normalizeCount(dependencyCounts?.isp_package_router_mappings),
  };

  const totalDependencies =
    normalizedDependencyCounts.pppoe_accounts + normalizedDependencyCounts.isp_package_router_mappings;

  if (totalDependencies > 0) {
    return {
      status: 'blocked',
      blocked: true,
      allowed: false,
      dependencyCounts: normalizedDependencyCounts,
      totalDependencies,
    };
  }

  return {
    status: 'allowed',
    blocked: false,
    allowed: true,
    dependencyCounts: normalizedDependencyCounts,
    totalDependencies,
  };
}

export function getPppProfileMutationErrorState(
  code: string | null | undefined,
): PppProfileMutationErrorState {
  if (code === 'mirror_sync_failed') {
    return {
      code: 'mirror_sync_failed',
      tone: 'warning',
      title: 'RouterOS changed, but the local mirror could not be refreshed',
      message:
        'The PPP profile was written on the router, but the mirrored database cache could not be refreshed yet.',
    };
  }

  return {
    code: 'router_write_failed',
    tone: 'error',
    title: 'RouterOS rejected the PPP profile change',
    message: 'The PPP profile was not changed on the router. Fix the router error and try again.',
  };
}

export function isPppProfileStaleTargetConflict(message: string | null | undefined): boolean {
  const normalized = String(message ?? '').toLowerCase();
  return normalized.includes('ppp profile no longer exists on router');
}

export function normalizePppProfilePayload(
  input: PppProfileFormPayloadInput,
): PppProfileNormalizedPayload {
  const normalize = (value: string) => {
    const trimmed = String(value ?? '').trim();
    return trimmed ? trimmed : null;
  };

  return {
    name: String(input.name ?? '').trim(),
    local_address: normalize(input.local_address),
    remote_address: normalize(input.remote_address),
    rate_limit: normalize(input.rate_limit),
    dns_server: normalize(input.dns_server),
    comment: normalize(input.comment),
    only_one: Boolean(input.only_one),
  };
}

export function getPppProfileOnlyOneState(value: boolean | null | undefined): PppProfileOnlyOneState {
  return {
    enabled: Boolean(value),
  };
}
