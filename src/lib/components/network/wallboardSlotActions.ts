import {
  mapInterfaceCatalogFromSnapshot,
  setSlotAt,
  updateSlotThresholdAt,
  clearSlotAt,
  type WallboardInterfaceCatalogItem,
  type WallboardSlotLike,
} from './wallboardSlots';
import {
  thresholdInputFromBps,
  thresholdBpsFromInput,
  type ThresholdUnit,
} from './wallboardThreshold';

export function openThresholdState(
  slots: (WallboardSlotLike | null)[],
  idx: number,
) {
  const slot = slots[idx];
  if (!slot) return null;

  const rx = thresholdInputFromBps(slot.warn_below_rx_bps ?? null);
  const tx = thresholdInputFromBps(slot.warn_below_tx_bps ?? null);

  return {
    thresholdIndex: idx,
    thWarnRxUnit: rx.unit,
    thWarnRxKbps: rx.value,
    thWarnTxUnit: tx.unit,
    thWarnTxKbps: tx.value,
  };
}

export function closeThresholdState() {
  return {
    thresholdIndex: null as number | null,
  };
}

export function saveThresholdState(args: {
  slots: (WallboardSlotLike | null)[];
  thresholdIndex: number | null;
  thWarnRxKbps: string;
  thWarnRxUnit: ThresholdUnit;
  thWarnTxKbps: string;
  thWarnTxUnit: ThresholdUnit;
}) {
  if (args.thresholdIndex == null) return null;

  const rxBps = thresholdBpsFromInput(args.thWarnRxKbps, args.thWarnRxUnit);
  const txBps = thresholdBpsFromInput(args.thWarnTxKbps, args.thWarnTxUnit);

  return {
    slotsAll: updateSlotThresholdAt(args.slots, args.thresholdIndex, rxBps, txBps),
    thresholdIndex: null as number | null,
  };
}

export function updateSlotThresholdState(
  slots: (WallboardSlotLike | null)[],
  idx: number,
  rxBps: number | null,
  txBps: number | null,
) {
  return {
    slotsAll: updateSlotThresholdAt(slots, idx, rxBps, txBps),
  };
}

export function setSlotState(
  slots: (WallboardSlotLike | null)[],
  idx: number,
  routerId: string,
  iface: string,
  warnBelowRxBps?: number | null,
  warnBelowTxBps?: number | null,
) {
  return {
    slotsAll: setSlotAt(slots, idx, routerId, iface, warnBelowRxBps, warnBelowTxBps),
    pickerIndex: null as number | null,
    pickerRouterId: null as string | null,
  };
}

export function clearSlotState(
  slots: (WallboardSlotLike | null)[],
  idx: number,
) {
  return {
    slotsAll: clearSlotAt(slots, idx),
  };
}

export async function loadInterfaceCatalog(args: {
  routerId: string;
  ifaceCatalog: Record<string, WallboardInterfaceCatalogItem[]>;
  fetchSnapshot: (routerId: string) => Promise<any>;
}) {
  if (args.ifaceCatalog[args.routerId]?.length) {
    return {
      ifaceCatalog: args.ifaceCatalog,
      changed: false,
    };
  }

  const snapshot = await args.fetchSnapshot(args.routerId);
  return {
    ifaceCatalog: {
      ...args.ifaceCatalog,
      [args.routerId]: mapInterfaceCatalogFromSnapshot(snapshot),
    },
    changed: true,
  };
}
