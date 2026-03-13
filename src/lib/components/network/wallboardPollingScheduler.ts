export function createWallboardPollingScheduler(args: {
  pollLiveOnce: () => void | Promise<void>;
  syncAlertsIncidents: () => void | Promise<void>;
  getPollMs: () => number;
  isPaused: () => boolean;
  isVisible: () => boolean;
  alertsIntervalMs?: number;
}) {
  let liveTick: ReturnType<typeof setTimeout> | null = null;
  let alertTick: ReturnType<typeof setInterval> | null = null;
  let liveLoopSeq = 0;

  function stopLiveTick() {
    if (liveTick) clearTimeout(liveTick);
    liveTick = null;
    liveLoopSeq++;
  }

  function stopAlertTick() {
    if (alertTick) clearInterval(alertTick);
    alertTick = null;
  }

  function scheduleNextLiveTick(loopSeq: number, delayMs: number) {
    liveTick = setTimeout(async () => {
      if (loopSeq !== liveLoopSeq) return;
      if (!args.isVisible() || args.isPaused()) {
        stopLiveTick();
        return;
      }
      try {
        await args.pollLiveOnce();
      } finally {
        if (loopSeq !== liveLoopSeq) return;
        const nextDelay = Math.max(250, args.getPollMs());
        scheduleNextLiveTick(loopSeq, nextDelay);
      }
    }, Math.max(250, delayMs));
  }

  function startLiveTick(nextPollMs?: number) {
    stopLiveTick();
    if (args.isPaused() || !args.isVisible()) return;
    const loopSeq = liveLoopSeq;
    const firstDelay = Math.max(250, nextPollMs ?? args.getPollMs());
    scheduleNextLiveTick(loopSeq, firstDelay);
  }

  function startAlertTick() {
    stopAlertTick();
    alertTick = setInterval(() => {
      if (!args.isVisible()) return;
      if (args.isPaused()) return;
      void args.syncAlertsIncidents();
    }, args.alertsIntervalMs ?? 10_000);
  }

  function refresh(nextPollMs?: number) {
    if (args.isPaused() || !args.isVisible()) {
      stopLiveTick();
      stopAlertTick();
      return;
    }
    startLiveTick(nextPollMs);
    startAlertTick();
  }

  function stopAll() {
    stopLiveTick();
    stopAlertTick();
  }

  return {
    refresh,
    stopAll,
    startLiveTick,
    startAlertTick,
  };
}

export function installVisibilityListener(onChange: (visible: boolean) => void) {
  if (typeof document === 'undefined') return null;
  const handler = () => onChange(!document.hidden);
  document.addEventListener('visibilitychange', handler);
  return () => document.removeEventListener('visibilitychange', handler);
}
