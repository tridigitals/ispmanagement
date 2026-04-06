export type IpPoolNameLike = {
  name: string;
};

export type IpPoolNextPoolFieldState = {
  mode: 'select' | 'manual';
  selectedValue: string;
  manualValue: string;
};

function normalizeText(value: string | null | undefined): string {
  return String(value ?? '').trim();
}

export function getAvailableNextPoolNames(
  pools: IpPoolNameLike[],
  currentPoolName?: string | null,
): string[] {
  const excludedName = normalizeText(currentPoolName);
  const uniqueNames = new Set<string>();

  for (const pool of pools) {
    const name = normalizeText(pool.name);
    if (!name || name === excludedName) continue;
    uniqueNames.add(name);
  }

  return Array.from(uniqueNames).sort((left, right) => left.localeCompare(right));
}

export function getInitialNextPoolFieldState(
  pools: IpPoolNameLike[],
  nextPoolValue?: string | null,
  currentPoolName?: string | null,
): IpPoolNextPoolFieldState {
  const normalizedNextPool = normalizeText(nextPoolValue);
  const availableNames = getAvailableNextPoolNames(pools, currentPoolName);

  if (!normalizedNextPool) {
    return {
      mode: 'select',
      selectedValue: '',
      manualValue: '',
    };
  }

  if (availableNames.includes(normalizedNextPool)) {
    return {
      mode: 'select',
      selectedValue: normalizedNextPool,
      manualValue: '',
    };
  }

  return {
    mode: 'manual',
    selectedValue: '',
    manualValue: normalizedNextPool,
  };
}

export function resolveNextPoolFieldValue(state: IpPoolNextPoolFieldState): string | null {
  const value =
    state.mode === 'manual' ? normalizeText(state.manualValue) : normalizeText(state.selectedValue);
  return value || null;
}
