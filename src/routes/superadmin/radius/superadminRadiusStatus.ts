export type ManagedRadiusUserRuntimeState = {
  is_provisioned: boolean;
  provisioning_error: string | null;
};

export type ManagedRadiusSessionRuntimeState = {
  status_type: string | null;
  ended_at: string | null;
};

export function getManagedRadiusUserStatus(
  user: ManagedRadiusUserRuntimeState,
): 'provisioned' | 'not_provisioned' {
  return user.is_provisioned ? 'provisioned' : 'not_provisioned';
}

export function getManagedRadiusUserBadgeTone(
  user: ManagedRadiusUserRuntimeState,
): 'good' | 'warn' | 'danger' {
  if (user.is_provisioned && !user.provisioning_error) return 'good';
  if (user.provisioning_error) return 'danger';
  return 'warn';
}

export function getManagedRadiusUserAttentionCount(
  users: ManagedRadiusUserRuntimeState[],
): number {
  return users.filter((user) => !user.is_provisioned || !!user.provisioning_error).length;
}

export function getManagedRadiusSessionStatus(
  session: ManagedRadiusSessionRuntimeState,
): 'online' | 'offline' {
  if (session.ended_at) return 'offline';
  const normalized = String(session.status_type || '')
    .trim()
    .toLowerCase();
  return normalized === 'stop' || normalized === 'accounting_off' || normalized === 'accounting-off'
    ? 'offline'
    : 'online';
}

export function getManagedRadiusSessionBadgeTone(
  session: ManagedRadiusSessionRuntimeState,
): 'good' | 'muted' {
  return getManagedRadiusSessionStatus(session) === 'online' ? 'good' : 'muted';
}

export function formatManagedRadiusSessionOctets(value: number | null | undefined): string {
  if (value === null || value === undefined) return '—';
  if (value < 1024) return `${value} B`;

  const units = ['KB', 'MB', 'GB', 'TB'];
  let next = value / 1024;
  let unitIndex = 0;
  while (next >= 1024 && unitIndex < units.length - 1) {
    next /= 1024;
    unitIndex += 1;
  }

  const decimals = next >= 10 ? 0 : 1;
  return `${next.toFixed(decimals)} ${units[unitIndex]}`;
}
