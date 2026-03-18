export function pruneSlotsByRouterIds<T extends { routerId: string }>(
  slots: (T | null)[],
  routerIds: string[],
): (T | null)[] {
  const ids = new Set(routerIds);
  return slots.map((slot) => (slot && ids.has(slot.routerId) ? slot : null));
}

export function filterRowsByStatus<T extends { is_online: boolean }>(
  rows: T[],
  status: 'all' | 'offline' | 'online',
): T[] {
  if (status === 'all') return rows;
  if (status === 'online') return rows.filter((r) => !!r.is_online);
  return rows.filter((r) => !r.is_online);
}

export function countActiveRouters<T extends { routerId: string }>(
  slots: (T | null)[],
) {
  const ids = new Set<string>();
  for (const s of slots) {
    if (s?.routerId) ids.add(s.routerId);
  }
  return ids.size;
}

export function resolveEffectivePollMs(basePollMs: number, activeRouters: number) {
  const floorMs =
    activeRouters >= 20 ? 8000 : activeRouters >= 12 ? 6000 : activeRouters >= 6 ? 4000 : 2000;
  return Math.max(basePollMs, floorMs);
}

export function resolveAdaptivePollMs(args: {
  basePollMs: number;
  activeRouters: number;
  lastPollDurationMs?: number;
  hasPollFailure?: boolean;
}) {
  let next = resolveEffectivePollMs(args.basePollMs, args.activeRouters);
  const duration = args.lastPollDurationMs ?? 0;

  if (duration >= 5000) next = Math.max(next, 10_000);
  else if (duration >= 3000) next = Math.max(next, 8000);
  else if (duration >= 1500) next = Math.max(next, 5000);

  if (args.hasPollFailure) next = Math.max(next, 7000);
  return Math.min(15_000, next);
}

export async function refreshWallboardRows<T extends { id: string }, S extends { routerId: string }>(args: {
  loadRouters: () => Promise<T[]>;
  slots: (S | null)[];
}) {
  const rows = await args.loadRouters();
  return {
    rows,
    slotsAll: pruneSlotsByRouterIds(args.slots, rows.map((row) => row.id)),
  };
}
