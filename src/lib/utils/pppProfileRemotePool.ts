export type PppProfileRemotePoolRow = {
  name: string;
};

function normalizeName(value: string | null | undefined): string {
  return String(value ?? '').trim();
}

export function getPppProfileRemotePoolOptions(pools: PppProfileRemotePoolRow[]): string[] {
  const uniqueNames = new Set<string>();

  for (const pool of pools) {
    const name = normalizeName(pool.name);
    if (!name) continue;
    uniqueNames.add(name);
  }

  return Array.from(uniqueNames).sort((left, right) => left.localeCompare(right));
}

export function getPppProfileRemotePoolValue(availableOptions: string[], remoteAddress: string | null | undefined): string {
  const normalizedValue = normalizeName(remoteAddress);
  if (!normalizedValue) return '';
  return availableOptions.includes(normalizedValue) ? normalizedValue : '';
}
