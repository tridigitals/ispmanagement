import type { MetricsRange } from './wallboardMetrics';

export type WallboardFullTab = 'live' | 'metrics';

export function toLocalInput(date: Date) {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  const hh = String(date.getHours()).padStart(2, '0');
  const mm = String(date.getMinutes()).padStart(2, '0');
  return `${y}-${m}-${d}T${hh}:${mm}`;
}

export function buildRangeInputs(range: MetricsRange, now: Date = new Date()) {
  if (range === 'custom') return null;
  let from = new Date(now);
  if (range === '24h') from.setHours(from.getHours() - 24);
  else if (range === '7d') from.setDate(from.getDate() - 7);
  else if (range === '30d') from.setDate(from.getDate() - 30);
  else if (range === 'month') from = new Date(now.getFullYear(), now.getMonth(), 1, 0, 0, 0, 0);

  return {
    fromLocal: toLocalInput(from),
    toLocal: toLocalInput(now),
  };
}

export function openFullPanelState(idx: number) {
  const range = '24h' as const;
  const inputs = buildRangeInputs(range) || { fromLocal: '', toLocal: '' };
  return {
    fullIndex: idx,
    fullTab: 'live' as const,
    metricsRange: range,
    metricsFromLocal: inputs.fromLocal,
    metricsToLocal: inputs.toLocal,
    metricsPointIdx: null as number | null,
    metricsZoomFrom: null as number | null,
    metricsZoomTo: null as number | null,
    metricsSelecting: false,
  };
}

export function closeFullPanelState() {
  return {
    fullIndex: null as number | null,
    fullMetricsLoading: false,
    fullMetricsError: null as string | null,
    fullMetricsRows: [] as any[],
    fullMetricsKey: '',
    fullMetricsLimit: 0,
    metricsFromLocal: '',
    metricsToLocal: '',
    metricsPointIdx: null as number | null,
    metricsZoomFrom: null as number | null,
    metricsZoomTo: null as number | null,
    metricsSelecting: false,
  };
}

export function switchFullTabState(tab: WallboardFullTab) {
  return {
    fullTab: tab,
    metricsPointIdx: null as number | null,
    metricsSelecting: false,
  };
}

export function setMetricsRangeState(next: MetricsRange, now: Date = new Date()) {
  const inputs = buildRangeInputs(next, now);
  return {
    metricsRange: next,
    metricsPointIdx: null as number | null,
    metricsZoomFrom: null as number | null,
    metricsZoomTo: null as number | null,
    metricsFromLocal: inputs?.fromLocal || '',
    metricsToLocal: inputs?.toLocal || '',
    shouldRefresh: next !== 'custom',
  };
}

export function clearMetricsZoomState() {
  return {
    metricsZoomFrom: null as number | null,
    metricsZoomTo: null as number | null,
    metricsPointIdx: null as number | null,
  };
}
