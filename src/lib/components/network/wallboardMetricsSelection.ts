export type MetricsChartRow = { ts: string; rx_bps: number; tx_bps: number };

export function beginMetricsSelection(
  e: PointerEvent,
): { selStart: number; selCurrent: number; selWidth: number } | null {
  if (e.button !== 0) return null;

  const el = e.currentTarget as HTMLElement | null;
  const rect = el?.getBoundingClientRect();
  if (!rect || rect.width <= 0) return null;

  const point = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
  try {
    el?.setPointerCapture?.(e.pointerId);
  } catch {
    // no-op
  }

  return {
    selStart: point,
    selCurrent: point,
    selWidth: rect.width,
  };
}

export function moveMetricsSelection(
  e: PointerEvent,
): { selCurrent: number; selWidth: number } | null {
  const el = e.currentTarget as HTMLElement | null;
  const rect = el?.getBoundingClientRect();
  if (!rect || rect.width <= 0) return null;

  return {
    selCurrent: Math.max(0, Math.min(rect.width, e.clientX - rect.left)),
    selWidth: rect.width,
  };
}

export function endMetricsSelection(
  e: PointerEvent,
  rows: MetricsChartRow[],
  selStart: number,
  selCurrent: number,
  selWidth: number,
  parseMetricTs: (ts?: string | null) => number | null,
): { zoomFrom: number; zoomTo: number } | null {
  const el = e.currentTarget as HTMLElement | null;
  try {
    el?.releasePointerCapture?.(e.pointerId);
  } catch {
    // no-op
  }

  const from = Math.min(selStart, selCurrent);
  const to = Math.max(selStart, selCurrent);
  if (!rows.length || selWidth <= 0 || to - from < 8) return null;

  const len = rows.length;
  const fromIdx = Math.max(0, Math.min(len - 1, Math.floor((from / selWidth) * (len - 1))));
  const toIdx = Math.max(0, Math.min(len - 1, Math.ceil((to / selWidth) * (len - 1))));
  const fromTs = parseMetricTs(rows[fromIdx]?.ts);
  const toTs = parseMetricTs(rows[toIdx]?.ts);
  if (fromTs == null || toTs == null) return null;

  return {
    zoomFrom: Math.min(fromTs, toTs),
    zoomTo: Math.max(fromTs, toTs),
  };
}

export function metricsHoverFromMouse(i: number, e: MouseEvent) {
  return {
    pointIdx: i,
    tooltipX: e.clientX + 14,
    tooltipY: e.clientY + 14,
  };
}

export function metricsHoverFromFocus(i: number, e: FocusEvent) {
  const el = e.currentTarget as HTMLElement | null;
  const rect = el?.getBoundingClientRect();
  if (!rect) {
    return {
      pointIdx: i,
      tooltipX: null,
      tooltipY: null,
    };
  }

  return {
    pointIdx: i,
    tooltipX: rect.left + rect.width / 2 + 10,
    tooltipY: rect.top + 10,
  };
}
