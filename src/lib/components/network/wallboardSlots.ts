export type WallboardSlotLike = {
  routerId: string;
  iface: string;
  warn_below_rx_bps?: number | null;
  warn_below_tx_bps?: number | null;
};

export type WallboardInterfaceCatalogItem = {
  name: string;
  interface_type?: string | null;
  running: boolean;
  disabled: boolean;
};

function ensureSlotIndex<T>(slots: (T | null)[], idx: number): (T | null)[] {
  if (idx < slots.length) return [...slots];
  return [...slots, ...Array.from({ length: idx + 1 - slots.length }, () => null)];
}

export function setSlotAt(
  slots: (WallboardSlotLike | null)[],
  idx: number,
  routerId: string,
  iface: string,
  warnBelowRxBps?: number | null,
  warnBelowTxBps?: number | null,
) {
  const next = ensureSlotIndex(slots, idx);
  next[idx] = {
    routerId,
    iface,
    warn_below_rx_bps: warnBelowRxBps ?? null,
    warn_below_tx_bps: warnBelowTxBps ?? null,
  };
  return next;
}

export function clearSlotAt(
  slots: (WallboardSlotLike | null)[],
  idx: number,
) {
  const next = ensureSlotIndex(slots, idx);
  next[idx] = null;
  return next;
}

export function updateSlotThresholdAt(
  slots: (WallboardSlotLike | null)[],
  idx: number,
  rxBps: number | null,
  txBps: number | null,
) {
  if (idx < 0 || idx >= slots.length) return slots;
  const cur = slots[idx];
  if (!cur) return slots;
  const next = [...slots];
  next[idx] = {
    ...cur,
    warn_below_rx_bps: rxBps,
    warn_below_tx_bps: txBps,
  };
  return next;
}

export function mapInterfaceCatalogFromSnapshot(snapshot: any): WallboardInterfaceCatalogItem[] {
  return ((snapshot?.interfaces || []) as any[])
    .map((it) => ({
      name: String(it?.name || ''),
      interface_type: (it?.interface_type ?? null) as string | null,
      running: !!it?.running,
      disabled: !!it?.disabled,
    }))
    .filter((it) => it.name);
}
