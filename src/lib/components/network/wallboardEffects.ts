type AlertLike = {
  id: string;
  severity?: string | null;
  title?: string | null;
  message?: string | null;
  router_id?: string;
};

export function buildMetricsAutoRefreshSig(args: {
  fullIndex: number | null;
  metricsRange: string;
  metricsFromLocal: string;
  metricsToLocal: string;
}) {
  if (args.fullIndex == null) return '';
  return `${args.fullIndex}|${args.metricsRange}|${args.metricsFromLocal}|${args.metricsToLocal}`;
}

export function resolveCriticalBeepEffect(args: {
  alerts: AlertLike[];
  lastSignature: string;
  lastBeepAt: number;
  soundEnabled: boolean;
  paused: boolean;
  now: number;
  minBeepGapMs?: number;
}) {
  const criticalIds = args.alerts
    .filter((a) => String(a.severity || '').toLowerCase() === 'critical')
    .map((a) => a.id)
    .sort();
  const signature = criticalIds.join(',');

  if (!signature) {
    return {
      signature: '',
      beepAt: args.lastBeepAt,
      shouldBeep: false,
    };
  }

  if (!args.soundEnabled || args.paused) {
    return {
      signature,
      beepAt: args.lastBeepAt,
      shouldBeep: false,
    };
  }

  if (!args.lastSignature) {
    return {
      signature,
      beepAt: args.lastBeepAt,
      shouldBeep: false,
    };
  }

  const minGap = args.minBeepGapMs ?? 8000;
  const changed = signature !== args.lastSignature;
  const gapOk = args.now - args.lastBeepAt >= minGap;

  return {
    signature,
    beepAt: changed && gapOk ? args.now : args.lastBeepAt,
    shouldBeep: changed && gapOk,
  };
}

export function resolveNewAlertIncidents(args: {
  alerts: AlertLike[];
  previousSnapshot: Record<string, string>;
}) {
  const current: Record<string, string> = {};
  for (const a of args.alerts) {
    current[a.id] = String(a.severity || '').toLowerCase();
  }

  // Bootstrap snapshot, do not emit history as new events.
  if (Object.keys(args.previousSnapshot).length === 0) {
    return {
      nextSnapshot: current,
      newIncidents: [] as { severity: 'critical' | 'warning'; message: string; routerId?: string }[],
    };
  }

  const newIncidents: { severity: 'critical' | 'warning'; message: string; routerId?: string }[] = [];
  for (const a of args.alerts) {
    if (args.previousSnapshot[a.id]) continue;
    const sev = String(a.severity || '').toLowerCase();
    if (sev === 'critical' || sev === 'warning') {
      newIncidents.push({
        severity: sev as 'critical' | 'warning',
        message: `${a.title || 'Alert'} · ${a.message || ''}`.trim(),
        routerId: a.router_id,
      });
    }
  }

  return { nextSnapshot: current, newIncidents };
}
