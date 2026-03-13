export type HistPoint = { ts: string; rx_bps: number; tx_bps: number };
export type MetricsRange = '24h' | '7d' | '30d' | 'month' | 'custom';
export type MetricsBucket = 'raw' | 'hour' | 'day';

function metricRowTsRaw(row: any) {
  return String(
    row?.ts ??
      row?.created_at ??
      row?.recorded_at ??
      row?.timestamp ??
      row?.time ??
      '',
  );
}

export function parseLocalDate(v: string): number | null {
  if (!v) return null;
  const ms = Date.parse(v);
  return Number.isFinite(ms) ? ms : null;
}

export function requiredMetricLimit(
  range: MetricsRange,
  fromLocal: string,
  toLocal: string,
) {
  if (range === '24h') return 400;
  if (range === '7d') return 2500;
  if (range === '30d') return 10000;
  if (range === 'month') return 10000;

  const fromMs = parseLocalDate(fromLocal);
  const toMs = parseLocalDate(toLocal);
  if (fromMs != null && toMs != null && toMs > fromMs) {
    const days = Math.ceil((toMs - fromMs) / (24 * 60 * 60 * 1000));
    if (days <= 2) return 800;
    if (days <= 8) return 3000;
    return 10000;
  }
  return 10000;
}

export function filterMetricsRowsByRange(
  fullMetricsRows: any[],
  fromLocal: string,
  toLocal: string,
  parseMetricTs: (ts?: string | null) => number | null,
) {
  const fromMs = parseLocalDate(fromLocal);
  const toMs = parseLocalDate(toLocal);
  if (fromMs == null && toMs == null) return fullMetricsRows;

  const filtered = fullMetricsRows.filter((row) => {
    const ts = parseMetricTs(metricRowTsRaw(row));
    if (ts == null) return false;
    if (fromMs != null && ts < fromMs) return false;
    if (toMs != null && ts > toMs) return false;
    return true;
  });

  if (filtered.length === 0 && fullMetricsRows.length > 0) return fullMetricsRows;
  return filtered;
}

export function buildHistPoints(
  rows: any[],
  parseMetricTs: (ts?: string | null) => number | null,
) {
  const asc = rows
    .map((row) => {
      const tsRaw = metricRowTsRaw(row);
      const tsMs = parseMetricTs(tsRaw);
      return { row, tsRaw, tsMs };
    })
    .filter((x) => x.tsMs != null)
    .sort((a, b) => (a.tsMs as number) - (b.tsMs as number));
  const out: HistPoint[] = [];

  let prevTs: number | null = null;
  let prevRxByte: number | null = null;
  let prevTxByte: number | null = null;

  for (const item of asc) {
    const row = item.row;
    const ts = item.tsRaw;
    const tsMs = item.tsMs as number;

    const directRx =
      typeof row?.rx_bps === 'number'
        ? Math.max(0, row.rx_bps)
        : typeof row?.rxBps === 'number'
          ? Math.max(0, row.rxBps)
          : null;
    const directTx =
      typeof row?.tx_bps === 'number'
        ? Math.max(0, row.tx_bps)
        : typeof row?.txBps === 'number'
          ? Math.max(0, row.txBps)
          : null;

    let rx = directRx;
    let tx = directTx;

    const curRxByte =
      typeof row?.rx_byte === 'number'
        ? row.rx_byte
        : typeof row?.rxByte === 'number'
          ? row.rxByte
          : typeof row?.rx_bytes === 'number'
            ? row.rx_bytes
            : null;
    const curTxByte =
      typeof row?.tx_byte === 'number'
        ? row.tx_byte
        : typeof row?.txByte === 'number'
          ? row.txByte
          : typeof row?.tx_bytes === 'number'
            ? row.tx_bytes
            : null;

    if (rx == null && curRxByte != null && prevRxByte != null && prevTs != null && tsMs > prevTs) {
      const delta = curRxByte - prevRxByte;
      if (delta >= 0) rx = Math.round((delta * 8 * 1000) / (tsMs - prevTs));
    }
    if (tx == null && curTxByte != null && prevTxByte != null && prevTs != null && tsMs > prevTs) {
      const delta = curTxByte - prevTxByte;
      if (delta >= 0) tx = Math.round((delta * 8 * 1000) / (tsMs - prevTs));
    }

    if (rx != null || tx != null) {
      out.push({ ts, rx_bps: rx ?? 0, tx_bps: tx ?? 0 });
    }

    if (curRxByte != null) prevRxByte = curRxByte;
    if (curTxByte != null) prevTxByte = curTxByte;
    prevTs = tsMs;
  }

  return out;
}

export function downsampleHistPoints(rows: HistPoint[], maxPoints: number = 120) {
  if (rows.length <= maxPoints) return rows;
  const step = rows.length / maxPoints;
  const out: HistPoint[] = [];

  for (let i = 0; i < maxPoints; i++) {
    const start = Math.floor(i * step);
    const end = Math.max(start + 1, Math.floor((i + 1) * step));
    const chunk = rows.slice(start, end);
    if (!chunk.length) continue;

    const rx = Math.round(chunk.reduce((acc, r) => acc + (r.rx_bps || 0), 0) / chunk.length);
    const tx = Math.round(chunk.reduce((acc, r) => acc + (r.tx_bps || 0), 0) / chunk.length);
    const ts = chunk[chunk.length - 1]?.ts || chunk[0]?.ts || '';
    out.push({ ts, rx_bps: rx, tx_bps: tx });
  }

  return out;
}

export function applyMetricsZoom(
  rows: HistPoint[],
  fromMs: number | null,
  toMs: number | null,
  parseMetricTs: (ts?: string | null) => number | null,
) {
  if (fromMs == null || toMs == null) return rows;
  const min = Math.min(fromMs, toMs);
  const max = Math.max(fromMs, toMs);
  const filtered = rows.filter((r) => {
    const ts = parseMetricTs(r.ts);
    return ts != null && ts >= min && ts <= max;
  });
  return filtered.length ? filtered : rows;
}

export function resolveMetricsBucket(
  range: MetricsRange,
  fromLocal: string,
  toLocal: string,
): MetricsBucket {
  if (range === '24h') return 'raw';
  if (range === '7d') return 'hour';
  if (range === '30d' || range === 'month') return 'day';

  const fromMs = parseLocalDate(fromLocal);
  const toMs = parseLocalDate(toLocal);
  if (fromMs != null && toMs != null && toMs > fromMs) {
    const days = (toMs - fromMs) / (24 * 60 * 60 * 1000);
    if (days <= 2) return 'raw';
    if (days <= 14) return 'hour';
    return 'day';
  }
  return 'hour';
}

export function aggregateHistPoints(
  rows: HistPoint[],
  bucket: MetricsBucket,
  parseMetricTs: (ts?: string | null) => number | null,
) {
  if (bucket === 'raw') return rows;

  const byKey = new Map<string, { ts: string; rxSum: number; txSum: number; count: number }>();
  for (const row of rows) {
    const ms = parseMetricTs(row.ts);
    if (ms == null) continue;
    const d = new Date(ms);
    const key =
      bucket === 'hour'
        ? `${d.getUTCFullYear()}-${d.getUTCMonth()}-${d.getUTCDate()}-${d.getUTCHours()}`
        : `${d.getUTCFullYear()}-${d.getUTCMonth()}-${d.getUTCDate()}`;

    const cur = byKey.get(key);
    if (!cur) {
      byKey.set(key, {
        ts: row.ts,
        rxSum: row.rx_bps,
        txSum: row.tx_bps,
        count: 1,
      });
    } else {
      cur.rxSum += row.rx_bps;
      cur.txSum += row.tx_bps;
      cur.count += 1;
      cur.ts = row.ts;
    }
  }

  return Array.from(byKey.values()).map((x) => ({
    ts: x.ts,
    rx_bps: Math.round(x.rxSum / Math.max(1, x.count)),
    tx_bps: Math.round(x.txSum / Math.max(1, x.count)),
  }));
}
