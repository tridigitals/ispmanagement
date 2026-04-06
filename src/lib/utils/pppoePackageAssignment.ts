export type PppoePackageAssignmentMapping = {
  package_id: string;
  package_name?: string | null;
  router_profile_name: string;
  address_pool: string | null;
};

export type PppoeCurrentAssignment = {
  router_profile_name?: string | null;
  remote_address?: string | null;
  address_pool?: string | null;
};

export type PppoeAssignmentPayload = {
  source: 'package' | 'account' | 'empty';
  hasPackageMapping: boolean;
  router_profile_name: string | null;
  remote_address: string | null;
  address_pool: string | null;
};

export type PppoeAssignmentPreview = {
  source: 'package' | 'account' | 'empty';
  hasPackageMapping: boolean;
  profileName: string;
  remoteAddress: string;
  addressPool: string;
};

function normalizeText(value: string | null | undefined): string {
  return String(value ?? '').trim();
}

function findPackageMapping(
  mappings: PppoePackageAssignmentMapping[],
  packageId: string | null | undefined,
): PppoePackageAssignmentMapping | null {
  const normalizedPackageId = normalizeText(packageId);
  if (!normalizedPackageId) return null;
  return mappings.find((mapping) => normalizeText(mapping.package_id) === normalizedPackageId) ?? null;
}

export function getPppoeAssignmentPayload({
  packageId,
  mappings,
  current,
}: {
  packageId: string | null | undefined;
  mappings: PppoePackageAssignmentMapping[];
  current: PppoeCurrentAssignment;
}): PppoeAssignmentPayload {
  const selectedMapping = findPackageMapping(mappings, packageId);
  if (selectedMapping) {
    const profileName = normalizeText(selectedMapping.router_profile_name);
    const addressPool = normalizeText(selectedMapping.address_pool);

    return {
      source: 'package',
      hasPackageMapping: true,
      router_profile_name: profileName || null,
      remote_address: null,
      address_pool: addressPool || null,
    };
  }

  const profileName = normalizeText(current.router_profile_name);
  const remoteAddress = normalizeText(current.remote_address);
  const addressPool = normalizeText(current.address_pool);
  const hasAnyAssignment = Boolean(profileName || remoteAddress || addressPool);

  return {
    source: hasAnyAssignment ? 'account' : 'empty',
    hasPackageMapping: false,
    router_profile_name: profileName || null,
    remote_address: remoteAddress || null,
    address_pool: addressPool || null,
  };
}

export function getPppoeAssignmentPreview({
  packageId,
  mappings,
  current,
}: {
  packageId: string | null | undefined;
  mappings: PppoePackageAssignmentMapping[];
  current: PppoeCurrentAssignment;
}): PppoeAssignmentPreview {
  const payload = getPppoeAssignmentPayload({ packageId, mappings, current });

  if (payload.source === 'package') {
    return {
      source: 'package',
      hasPackageMapping: true,
      profileName: payload.router_profile_name || '',
      remoteAddress: payload.address_pool || '',
      addressPool: payload.address_pool || '',
    };
  }

  return {
    source: payload.source,
    hasPackageMapping: false,
    profileName: payload.router_profile_name || '',
    remoteAddress: payload.remote_address || payload.address_pool || '',
    addressPool: payload.address_pool || '',
  };
}
