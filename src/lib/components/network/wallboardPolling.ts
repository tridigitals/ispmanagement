export type WallboardSlotRef = {
  routerId: string;
  iface: string;
};

export type WallboardLiveCounter = {
  name: string;
  running: boolean;
  disabled: boolean;
  rx_byte: number;
  tx_byte: number;
};

export type WallboardLiveRate = {
  rx_bps: number | null;
  tx_bps: number | null;
  last_seen_at: number;
};

export type WallboardRouterPollState = {
  fails: number;
  nextRetryAt: number;
  lastErrorAt: number | null;
  lastSuccessAt: number | null;
};

export async function pollWallboardLiveOnce(args: {
  paused: boolean;
  documentHidden: boolean;
  slotsAll: (WallboardSlotRef | null)[];
  routerPollState: Record<string, WallboardRouterPollState>;
  liveRates: Record<string, Record<string, WallboardLiveRate>>;
  series: Record<string, Record<string, { rx: number[]; tx: number[] }>>;
  lastBytes: Map<string, { rx: number; tx: number; at: number }>;
  loadInterfaceLive: (routerId: string, ifaceNames: string[]) => Promise<WallboardLiveCounter[]>;
  onRecovered: (routerId: string) => void;
  onPollError: (routerId: string, fails: number) => void;
  now?: () => number;
  maxRouters?: number;
  maxInterfacesPerRouter?: number;
}) {
  if (args.documentHidden || args.paused) return null;

  const wanted = args.slotsAll.filter(Boolean) as WallboardSlotRef[];
  if (wanted.length === 0) return null;

  const byRouter = new Map<string, Set<string>>();
  for (const s of wanted) {
    if (!s.routerId || !s.iface) continue;
    let set = byRouter.get(s.routerId);
    if (!set) {
      set = new Set<string>();
      byRouter.set(s.routerId, set);
    }
    set.add(s.iface);
  }

  const nowFn = args.now ?? Date.now;
  const maxRouters = args.maxRouters ?? 12;
  const maxInterfaces = args.maxInterfacesPerRouter ?? 12;

  const nextLiveRates: Record<string, Record<string, WallboardLiveRate>> = { ...args.liveRates };
  const nextSeries: Record<string, Record<string, { rx: number[]; tx: number[] }>> = { ...args.series };
  let nextRouterPollState = args.routerPollState;

  // Sequential polling keeps router API load predictable.
  const routerIds = Array.from(byRouter.keys()).slice(0, maxRouters);
  for (const routerId of routerIds) {
    const ifaceNames = Array.from(byRouter.get(routerId) || []).filter(Boolean).slice(0, maxInterfaces);
    if (!ifaceNames.length) continue;

    const before = nowFn();
    const currentState = nextRouterPollState[routerId];
    if (currentState && currentState.nextRetryAt > before) continue;

    try {
      const counters = await args.loadInterfaceLive(routerId, ifaceNames);
      const now = nowFn();

      const rateMap: Record<string, WallboardLiveRate> = { ...(nextLiveRates[routerId] || {}) };
      const seriesMap: Record<string, { rx: number[]; tx: number[] }> = { ...(nextSeries[routerId] || {}) };
      nextLiveRates[routerId] = rateMap;
      nextSeries[routerId] = seriesMap;

      for (const c of counters || []) {
        const key = `${routerId}:${c.name}`;
        const prev = args.lastBytes.get(key);
        const rx = c.rx_byte ?? 0;
        const tx = c.tx_byte ?? 0;

        let rxBps: number | null = null;
        let txBps: number | null = null;
        if (prev && now > prev.at) {
          const dt = (now - prev.at) / 1000;
          rxBps = Math.max(0, Math.round((rx - prev.rx) / dt) * 8);
          txBps = Math.max(0, Math.round((tx - prev.tx) / dt) * 8);
        }

        args.lastBytes.set(key, { rx, tx, at: now });
        rateMap[c.name] = { rx_bps: rxBps, tx_bps: txBps, last_seen_at: now };

        const prevBuf = seriesMap[c.name] || { rx: [], tx: [] };
        const rxBuf = [...prevBuf.rx, rxBps ?? 0];
        const txBuf = [...prevBuf.tx, txBps ?? 0];
        if (rxBuf.length > 60) rxBuf.splice(0, rxBuf.length - 60);
        if (txBuf.length > 60) txBuf.splice(0, txBuf.length - 60);
        seriesMap[c.name] = { rx: rxBuf, tx: txBuf };
      }

      if (currentState && currentState.fails >= 3) {
        args.onRecovered(routerId);
      }
      if (currentState && currentState.fails > 0) {
        nextRouterPollState = {
          ...nextRouterPollState,
          [routerId]: {
            fails: 0,
            nextRetryAt: 0,
            lastErrorAt: currentState.lastErrorAt,
            lastSuccessAt: now,
          },
        };
      }
    } catch {
      const prev = nextRouterPollState[routerId] || {
        fails: 0,
        nextRetryAt: 0,
        lastErrorAt: null,
        lastSuccessAt: null,
      };
      const fails = prev.fails + 1;
      const backoffMs = Math.min(30_000, 1000 * 2 ** Math.min(fails, 5));
      const nextRetryAt = nowFn() + backoffMs;
      nextRouterPollState = {
        ...nextRouterPollState,
        [routerId]: {
          fails,
          nextRetryAt,
          lastErrorAt: nowFn(),
          lastSuccessAt: prev.lastSuccessAt,
        },
      };

      if (fails === 3 || fails === 5 || fails % 10 === 0) {
        args.onPollError(routerId, fails);
      }
    }
  }

  return {
    liveRates: nextLiveRates,
    series: nextSeries,
    routerPollState: nextRouterPollState,
    renderNow: nowFn(),
  };
}
